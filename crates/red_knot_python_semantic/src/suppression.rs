use crate::lint::{Level, LintRegistry, LintStatus};
use crate::types::{TypeCheckDiagnostic, TypeCheckDiagnostics};
use crate::{declare_lint, lint::LintId, Db};
use ruff_db::diagnostic::DiagnosticId;
use ruff_db::{files::File, parsed::parsed_module, source::source_text};
use ruff_python_parser::TokenKind;
use ruff_python_trivia::Cursor;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::fmt::Formatter;

declare_lint! {
    /// ## What it does
    /// Checks for `type: ignore` or `knot: ignore` directives that are no longer applicable.
    ///
    /// ## Why is this bad?
    /// A `type: ignore` directive that no longer matches any diagnostic violations is likely
    /// included by mistake, and should be removed to avoid confusion.
    ///
    /// ## Examples
    /// ```py
    /// a = 20 / 2  # knot: ignore[division-by-zero]
    /// ```
    ///
    /// Use instead:
    ///
    /// ```py
    /// a = 20 / 2
    /// ```
    pub(crate) static UNUSED_IGNORE_COMMENT = {
        summary: "detects unused `type: ignore` comments",
        status: LintStatus::preview("1.0.0"),
        default_level: Level::Warn,
    }
}

#[salsa::tracked(return_ref)]
pub(crate) fn suppressions(db: &dyn Db, file: File) -> Suppressions {
    let parsed = parsed_module(db.upcast(), file);
    let source = source_text(db.upcast(), file);

    let mut builder = SuppressionsBuilder::new(&source, db.lint_registry());
    let mut line_start = TextSize::default();

    for token in parsed.tokens() {
        if !token.kind().is_trivia() {
            builder.set_seen_non_trivia_token();
        }

        match token.kind() {
            TokenKind::Comment => {
                let parser = SuppressionParser::new(&source, token.range());

                for comment in parser {
                    builder.add_comment(&comment, TextRange::new(line_start, token.end()));
                }
            }
            TokenKind::Newline | TokenKind::NonLogicalNewline => {
                line_start = token.end();
            }
            _ => {}
        }
    }

    builder.finish()
}

/// Checks for unused suppression comments in `file` and
/// adds diagnostic for each of them to `diagnostics`.
///
/// Does nothing if the [`UNUSED_IGNORE_COMMENT`] rule is disabled.
pub(crate) fn check_unused_suppressions(
    db: &dyn Db,
    file: File,
    diagnostics: &mut TypeCheckDiagnostics,
) {
    let Some(severity) = db
        .rule_selection()
        .severity(LintId::of(&UNUSED_IGNORE_COMMENT))
    else {
        return;
    };

    let all = suppressions(db, file);
    let mut unused = Vec::with_capacity(
        all.file
            .len()
            .saturating_add(all.line.len())
            .saturating_sub(diagnostics.used_len()),
    );

    // Collect all suppressions that are unused after type-checking.
    for suppression in all {
        if diagnostics.is_used(suppression.id()) {
            continue;
        }

        // `unused-ignore-comment` diagnostics can only be suppressed by specifying a
        // code. This is necessary because every `type: ignore` would implicitly also
        // suppress its own unused-ignore-comment diagnostic.
        if let Some(unused_suppression) = all
            .lint_suppressions(suppression.range, LintId::of(&UNUSED_IGNORE_COMMENT))
            .find(|unused_ignore_suppression| unused_ignore_suppression.target.is_lint())
        {
            // A `unused-ignore-comment` suppression can't ignore itself.
            // It can only ignore other suppressions.
            if unused_suppression.id() != suppression.id() {
                diagnostics.mark_used(unused_suppression.id());
                continue;
            }
        }

        unused.push(suppression);
    }

    for suppression in unused {
        // This looks silly but it's necessary to check again if a `unused-ignore-comment` is indeed unused
        // in case the "unused" directive comes after it:
        // ```py
        // a = 10 / 2  # knot: ignore[unused-ignore-comment, division-by-zero]
        // ```
        if diagnostics.is_used(suppression.id()) {
            continue;
        }

        let message = match suppression.target {
            SuppressionTarget::All => {
                format!("Unused blanket `{}` directive", suppression.kind)
            }
            SuppressionTarget::Lint(lint) => {
                format!(
                    "Unused `{kind}` directive: '{code}'",
                    kind = suppression.kind,
                    code = lint.name()
                )
            }
            SuppressionTarget::Empty => {
                format!("Unused `{}` without a code", suppression.kind)
            }
        };
        diagnostics.push(TypeCheckDiagnostic {
            id: DiagnosticId::Lint(UNUSED_IGNORE_COMMENT.name()),
            message,
            range: suppression.range,
            severity,
            file,
        });
    }
}

/// The suppressions of a single file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Suppressions {
    /// Suppressions that apply to the entire file.
    ///
    /// The suppressions are sorted by [`Suppression::comment_range`] and the [`Suppression::suppressed_range`]
    /// spans the entire file.
    ///
    /// For now, this is limited to `type: ignore` comments.
    file: Vec<Suppression>,

    /// Suppressions that apply to a specific line (or lines).
    ///
    /// Comments with multiple codes create multiple [`Suppression`]s that all share the same [`Suppression::comment_range`].
    ///
    /// The suppressions are sorted by [`Suppression::range`] (which implies [`Suppression::comment_range`]).
    line: Vec<Suppression>,
}

impl Suppressions {
    pub(crate) fn find_suppression(&self, range: TextRange, id: LintId) -> Option<&Suppression> {
        self.lint_suppressions(range, id).next()
    }

    /// Returns all suppressions for the given lint
    fn lint_suppressions(
        &self,
        range: TextRange,
        id: LintId,
    ) -> impl Iterator<Item = &Suppression> + '_ {
        self.file
            .iter()
            .chain(self.line_suppressions(range))
            .filter(move |suppression| suppression.matches(id))
    }

    /// Returns the line-level suppressions that apply for `range`.
    ///
    /// A suppression applies for the given range if it contains the range's
    /// start or end offset. This means the suppression is on the same line
    /// as the diagnostic's start or end.
    fn line_suppressions(&self, range: TextRange) -> impl Iterator<Item = &Suppression> + '_ {
        // First find the index of the suppression comment that ends right before the range
        // starts. This allows us to skip suppressions that are not relevant for the range.
        let end_offset = self
            .line
            .binary_search_by_key(&range.start(), |suppression| {
                suppression.suppressed_range.end()
            })
            .unwrap_or_else(|index| index);

        // From here, search the remaining suppression comments for one that
        // contains the range's start or end offset. Stop the search
        // as soon as the suppression's range and the range no longer overlap.
        self.line[end_offset..]
            .iter()
            // Stop searching if the suppression starts after the range we're looking for.
            .take_while(move |suppression| range.end() >= suppression.suppressed_range.start())
            .filter(move |suppression| {
                // Don't use intersect to avoid that suppressions on inner-expression
                // ignore errors for outer expressions
                suppression.suppressed_range.contains(range.start())
                    || suppression.suppressed_range.contains(range.end())
            })
    }

    fn iter(&self) -> SuppressionsIter {
        self.file.iter().chain(&self.line)
    }
}

pub(crate) type SuppressionsIter<'a> =
    std::iter::Chain<std::slice::Iter<'a, Suppression>, std::slice::Iter<'a, Suppression>>;

impl<'a> IntoIterator for &'a Suppressions {
    type Item = &'a Suppression;
    type IntoIter = SuppressionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A `type: ignore` or `knot: ignore` suppression.
///
/// Suppression comments that suppress multiple codes
/// create multiple suppressions: one for every code.
/// They all share the same `comment_range`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Suppression {
    target: SuppressionTarget,
    kind: SuppressionKind,

    /// The range of this specific suppression.
    /// This is the same as `comment_range` except for suppression comments that suppress multiple
    /// codes. For those, the range is limited to the specific code.
    range: TextRange,

    /// The range of the suppression comment.
    comment_range: TextRange,

    /// The range for which this suppression applies.
    /// Most of the time, this is the range of the comment's line.
    /// However, there are few cases where the range gets expanded to
    /// cover multiple lines:
    /// * multiline strings: `expr + """multiline\nstring"""  # type: ignore`
    /// * line continuations: `expr \ + "test"  # type: ignore`
    suppressed_range: TextRange,
}

impl Suppression {
    fn matches(&self, tested_id: LintId) -> bool {
        match self.target {
            SuppressionTarget::All => true,
            SuppressionTarget::Lint(suppressed_id) => tested_id == suppressed_id,
            SuppressionTarget::Empty => false,
        }
    }

    pub(crate) fn id(&self) -> FileSuppressionId {
        FileSuppressionId(self.range)
    }
}

/// Unique ID for a suppression in a file.
///
/// ## Implementation
/// The wrapped `TextRange` is the suppression's range.
/// This is unique enough because it is its exact
/// location in the source.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct FileSuppressionId(TextRange);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SuppressionTarget {
    /// Suppress all lints
    All,

    /// Suppress the lint with the given id
    Lint(LintId),

    /// Suppresses no lint, e.g. `knot: ignore[]`
    Empty,
}

impl SuppressionTarget {
    const fn is_lint(self) -> bool {
        matches!(self, SuppressionTarget::Lint(_))
    }
}

struct SuppressionsBuilder<'a> {
    lint_registry: &'a LintRegistry,
    source: &'a str,

    /// `type: ignore` comments at the top of the file before any non-trivia code apply to the entire file.
    /// This boolean tracks if there has been any non trivia token.
    seen_non_trivia_token: bool,

    line: Vec<Suppression>,
    file: Vec<Suppression>,
}

impl<'a> SuppressionsBuilder<'a> {
    fn new(source: &'a str, lint_registry: &'a LintRegistry) -> Self {
        Self {
            source,
            lint_registry,
            seen_non_trivia_token: false,
            line: Vec::new(),
            file: Vec::new(),
        }
    }

    fn set_seen_non_trivia_token(&mut self) {
        self.seen_non_trivia_token = true;
    }

    fn finish(mut self) -> Suppressions {
        self.line.shrink_to_fit();
        self.file.shrink_to_fit();

        Suppressions {
            file: self.file,
            line: self.line,
        }
    }

    fn add_comment(&mut self, comment: &SuppressionComment, line_range: TextRange) {
        let (suppressions, suppressed_range) =
            // `type: ignore` comments at the start of the file apply to the entire range.
            // > A # type: ignore comment on a line by itself at the top of a file, before any docstrings,
            // > imports, or other executable code, silences all errors in the file.
            // > Blank lines and other comments, such as shebang lines and coding cookies,
            // > may precede the # type: ignore comment.
            // > https://typing.readthedocs.io/en/latest/spec/directives.html#type-ignore-comments
            if comment.kind.is_type_ignore() && !self.seen_non_trivia_token {
                (
                    &mut self.file,
                    TextRange::new(0.into(), self.source.text_len()),
                )
            } else {
                (
                    &mut self.line,
                    line_range,
                )
            };

        match comment.codes.as_deref() {
            // `type: ignore`
            None => {
                suppressions.push(Suppression {
                    target: SuppressionTarget::All,
                    kind: comment.kind,
                    comment_range: comment.range,
                    range: comment.range,
                    suppressed_range,
                });
            }

            // `type: ignore[..]`
            // The suppression applies to all lints if it is a `type: ignore`
            // comment. `type: ignore` apply to all lints for better mypy compatibility.
            Some(_) if comment.kind.is_type_ignore() => {
                suppressions.push(Suppression {
                    target: SuppressionTarget::All,
                    kind: comment.kind,
                    comment_range: comment.range,
                    range: comment.range,
                    suppressed_range,
                });
            }

            // `knot: ignore[]`
            Some([]) => {
                suppressions.push(Suppression {
                    target: SuppressionTarget::Empty,
                    kind: comment.kind,
                    range: comment.range,
                    comment_range: comment.range,
                    suppressed_range,
                });
            }

            // `knot: ignore[a, b]`
            Some(codes) => {
                for code_range in codes {
                    let code = &self.source[*code_range];
                    match self.lint_registry.get(code) {
                        Ok(lint) => {
                            let range = if codes.len() == 1 {
                                comment.range
                            } else {
                                *code_range
                            };

                            suppressions.push(Suppression {
                                target: SuppressionTarget::Lint(lint),
                                kind: comment.kind,
                                range,
                                comment_range: comment.range,
                                suppressed_range,
                            });
                        }
                        Err(error) => {
                            tracing::debug!("Invalid suppression: {error}");
                            // TODO(micha): Handle invalid lint codes
                        }
                    }
                }
            }
        }
    }
}

struct SuppressionParser<'src> {
    cursor: Cursor<'src>,
    range: TextRange,
}

impl<'src> SuppressionParser<'src> {
    fn new(source: &'src str, range: TextRange) -> Self {
        let cursor = Cursor::new(&source[range]);

        Self { cursor, range }
    }

    fn parse_comment(&mut self) -> Option<SuppressionComment> {
        let comment_start = self.offset();
        self.cursor.start_token();

        if !self.cursor.eat_char('#') {
            return None;
        }

        self.eat_whitespace();

        // type: ignore[code]
        // ^^^^^^^^^^^^
        let kind = self.eat_kind()?;

        let has_trailing_whitespace = self.eat_whitespace();

        // type: ignore[code1, code2]
        //             ^^^^^^
        let codes = self.eat_codes();

        if self.cursor.is_eof() || codes.is_some() || has_trailing_whitespace {
            // Consume the comment until its end or until the next "sub-comment" starts.
            self.cursor.eat_while(|c| c != '#');
            Some(SuppressionComment {
                kind,
                codes,
                range: TextRange::at(comment_start, self.cursor.token_len()),
            })
        } else {
            None
        }
    }

    fn eat_kind(&mut self) -> Option<SuppressionKind> {
        let kind = if self.cursor.as_str().starts_with("type") {
            SuppressionKind::TypeIgnore
        } else if self.cursor.as_str().starts_with("knot") {
            SuppressionKind::Knot
        } else {
            return None;
        };

        self.cursor.skip_bytes(kind.len_utf8());

        self.eat_whitespace();

        if !self.cursor.eat_char(':') {
            return None;
        }

        self.eat_whitespace();

        if !self.cursor.as_str().starts_with("ignore") {
            return None;
        }

        self.cursor.skip_bytes("ignore".len());

        Some(kind)
    }

    fn eat_codes(&mut self) -> Option<SmallVec<[TextRange; 2]>> {
        if !self.cursor.eat_char('[') {
            return None;
        }

        let mut codes: SmallVec<[TextRange; 2]> = smallvec![];

        loop {
            if self.cursor.is_eof() {
                return None;
            }

            self.eat_whitespace();

            // `knot: ignore[]` or `knot: ignore[a,]`
            if self.cursor.eat_char(']') {
                break Some(codes);
            }

            let code_start = self.offset();
            if !self.eat_word() {
                return None;
            }

            codes.push(TextRange::new(code_start, self.offset()));

            self.eat_whitespace();

            if !self.cursor.eat_char(',') {
                self.eat_whitespace();

                if self.cursor.eat_char(']') {
                    break Some(codes);
                }
                // `knot: ignore[a b]
                return None;
            }
        }
    }

    fn eat_whitespace(&mut self) -> bool {
        if self.cursor.eat_if(char::is_whitespace) {
            self.cursor.eat_while(char::is_whitespace);
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self) -> bool {
        if self.cursor.eat_if(char::is_alphabetic) {
            self.cursor
                .eat_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-'));
            true
        } else {
            false
        }
    }

    fn offset(&self) -> TextSize {
        self.range.start() + self.range.len() - self.cursor.text_len()
    }
}

impl Iterator for SuppressionParser<'_> {
    type Item = SuppressionComment;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cursor.is_eof() {
                return None;
            }

            if let Some(suppression) = self.parse_comment() {
                return Some(suppression);
            }

            self.cursor.eat_while(|c| c != '#');
        }
    }
}

/// A single parsed suppression comment.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SuppressionComment {
    /// The range of the suppression comment.
    ///
    /// This can be a sub-range of the comment token if the comment token contains multiple `#` tokens:
    /// ```py
    /// # fmt: off # type: ignore
    ///            ^^^^^^^^^^^^^^
    /// ```
    range: TextRange,

    kind: SuppressionKind,

    /// The ranges of the codes in the optional `[...]`.
    /// `None` for comments that don't specify any code.
    ///
    /// ```py
    /// # type: ignore[unresolved-reference, invalid-exception-caught]
    ///                ^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    codes: Option<SmallVec<[TextRange; 2]>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SuppressionKind {
    TypeIgnore,
    Knot,
}

impl SuppressionKind {
    const fn is_type_ignore(self) -> bool {
        matches!(self, SuppressionKind::TypeIgnore)
    }

    fn len_utf8(self) -> usize {
        match self {
            SuppressionKind::TypeIgnore => "type".len(),
            SuppressionKind::Knot => "knot".len(),
        }
    }
}

impl fmt::Display for SuppressionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SuppressionKind::TypeIgnore => f.write_str("type: ignore"),
            SuppressionKind::Knot => f.write_str("knot: ignore"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::suppression::{SuppressionComment, SuppressionParser};
    use insta::assert_debug_snapshot;
    use ruff_text_size::{TextLen, TextRange};
    use std::fmt;
    use std::fmt::Formatter;

    #[test]
    fn type_ignore_no_codes() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore",
                kind: TypeIgnore,
                codes: [],
            },
        ]
        "##
        );
    }

    #[test]
    fn type_ignore_explanation() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore I tried but couldn't figure out the proper type",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore I tried but couldn't figure out the proper type",
                kind: TypeIgnore,
                codes: [],
            },
        ]
        "##
        );
    }

    #[test]
    fn fmt_comment_before_type_ignore() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# fmt: off   # type: ignore",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore",
                kind: TypeIgnore,
                codes: [],
            },
        ]
        "##
        );
    }

    #[test]
    fn type_ignore_before_fmt_off() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore  # fmt: off",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore  ",
                kind: TypeIgnore,
                codes: [],
            },
        ]
        "##
        );
    }

    #[test]
    fn multiple_type_ignore_comments() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore[a]  # type: ignore[b]",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore[a]  ",
                kind: TypeIgnore,
                codes: [
                    "a",
                ],
            },
            SuppressionComment {
                text: "# type: ignore[b]",
                kind: TypeIgnore,
                codes: [
                    "b",
                ],
            },
        ]
        "##
        );
    }

    #[test]
    fn invalid_type_ignore_valid_type_ignore() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore[a  # type: ignore[b]",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore[b]",
                kind: TypeIgnore,
                codes: [
                    "b",
                ],
            },
        ]
        "##
        );
    }

    #[test]
    fn valid_type_ignore_invalid_type_ignore() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore[a]  # type: ignoreeee",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore[a]  ",
                kind: TypeIgnore,
                codes: [
                    "a",
                ],
            },
        ]
        "##
        );
    }

    #[test]
    fn type_ignore_multiple_codes() {
        assert_debug_snapshot!(
            SuppressionComments::new(
                "# type: ignore[invalid-exception-raised, invalid-exception-caught]",
            ),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore[invalid-exception-raised, invalid-exception-caught]",
                kind: TypeIgnore,
                codes: [
                    "invalid-exception-raised",
                    "invalid-exception-caught",
                ],
            },
        ]
        "##
        );
    }

    #[test]
    fn type_ignore_single_code() {
        assert_debug_snapshot!(
            SuppressionComments::new("# type: ignore[invalid-exception-raised]",),
            @r##"
        [
            SuppressionComment {
                text: "# type: ignore[invalid-exception-raised]",
                kind: TypeIgnore,
                codes: [
                    "invalid-exception-raised",
                ],
            },
        ]
        "##
        );
    }

    struct SuppressionComments<'a> {
        source: &'a str,
    }

    impl<'a> SuppressionComments<'a> {
        fn new(source: &'a str) -> Self {
            Self { source }
        }
    }

    impl fmt::Debug for SuppressionComments<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut list = f.debug_list();

            for comment in SuppressionParser::new(
                self.source,
                TextRange::new(0.into(), self.source.text_len()),
            ) {
                list.entry(&comment.debug(self.source));
            }

            list.finish()
        }
    }

    impl SuppressionComment {
        fn debug<'a>(&'a self, source: &'a str) -> DebugSuppressionComment<'a> {
            DebugSuppressionComment {
                source,
                comment: self,
            }
        }
    }

    struct DebugSuppressionComment<'a> {
        source: &'a str,
        comment: &'a SuppressionComment,
    }

    impl fmt::Debug for DebugSuppressionComment<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            struct DebugCodes<'a> {
                source: &'a str,
                codes: &'a [TextRange],
            }

            impl fmt::Debug for DebugCodes<'_> {
                fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                    let mut f = f.debug_list();

                    for code in self.codes {
                        f.entry(&&self.source[*code]);
                    }

                    f.finish()
                }
            }

            f.debug_struct("SuppressionComment")
                .field("text", &&self.source[self.comment.range])
                .field("kind", &self.comment.kind)
                .field(
                    "codes",
                    &DebugCodes {
                        source: self.source,
                        codes: self.comment.codes.as_deref().unwrap_or_default(),
                    },
                )
                .finish()
        }
    }
}
