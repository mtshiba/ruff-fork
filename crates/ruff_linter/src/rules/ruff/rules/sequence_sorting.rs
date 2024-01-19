/// Utilities for sorting constant lists of string literals.
///
/// Examples where these are useful:
/// - Sorting `__all__` in the global scope,
/// - Sorting `__slots__` or `__match_args__` in a class scope
use std::borrow::Cow;
use std::cmp::Ordering;

use ruff_python_ast as ast;
use ruff_python_codegen::Stylist;
use ruff_python_parser::{lexer, Mode, Tok, TokenKind};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::Locator;
use ruff_text_size::{Ranged, TextRange, TextSize};

use is_macro;
use itertools::Itertools;

/// An enumeration of the various kinds of sequences for which Python has
/// [display literals](https://docs.python.org/3/reference/expressions.html#displays-for-lists-sets-and-dictionaries).
///
/// (I'm aware a set isn't actually a "sequence",
/// *but* for our purposes it's conceptually a sequence,
/// since in terms of the AST structure it's almost identical
/// to tuples/lists.)
///
/// Whereas lists, dicts and sets are always parenthesized
/// (e.g. lists always start with `[` and end with `]`),
/// single-line tuples *can* be unparenthesized.
/// We keep the original AST node around for the
/// Tuple variant so that this can be queried later.
#[derive(Debug)]
pub(super) enum SequenceKind<'a> {
    List,
    Set,
    Tuple(&'a ast::ExprTuple),
}

impl SequenceKind<'_> {
    fn surrounding_parens(&self, source: &str) -> (&'static str, &'static str) {
        match self {
            Self::List => ("[", "]"),
            Self::Set => ("{", "}"),
            Self::Tuple(ast_node) => {
                if ast_node.is_parenthesized(source) {
                    ("(", ")")
                } else {
                    ("", "")
                }
            }
        }
    }

    const fn opening_token_for_multiline_definition(&self) -> TokenKind {
        match self {
            Self::List => TokenKind::Lsqb,
            Self::Set => TokenKind::Lbrace,
            Self::Tuple(_) => TokenKind::Lpar,
        }
    }

    const fn closing_token_for_multiline_definition(&self) -> TokenKind {
        match self {
            Self::List => TokenKind::Rsqb,
            Self::Set => TokenKind::Rbrace,
            Self::Tuple(_) => TokenKind::Rpar,
        }
    }
}

/// An enumeration of the various kinds of
/// [display literals](https://docs.python.org/3/reference/expressions.html#displays-for-lists-sets-and-dictionaries)
/// Python provides for builtin containers.
#[derive(Debug, is_macro::Is)]
pub(super) enum DisplayKind<'a> {
    Sequence(SequenceKind<'a>),
    Dict { values: &'a Vec<ast::Expr> },
}

/// Create a string representing a fixed-up single-line
/// definition of `__all__` or `__slots__` (etc.),
/// that can be inserted into the
/// source code as a `range_replacement` autofix.
pub(super) fn sort_single_line_elements_sequence<F>(
    kind: &SequenceKind,
    elts: &[ast::Expr],
    elements: &[&str],
    locator: &Locator,
    mut cmp_fn: F,
) -> String
where
    F: FnMut(&str, &str) -> Ordering,
{
    assert_eq!(elts.len(), elements.len());
    let (opening_paren, closing_paren) = kind.surrounding_parens(locator.contents());
    let last_item_index = elements.len().saturating_sub(1);
    let mut result = String::from(opening_paren);

    let mut element_pairs = elements.iter().zip(elts).collect_vec();
    element_pairs.sort_by(|(elem1, _), (elem2, _)| cmp_fn(elem1, elem2));
    // We grab the original source-code ranges using `locator.slice()`
    // rather than using the expression generator, as this approach allows
    // us to easily preserve stylistic choices in the original source code
    // such as whether double or single quotes were used.
    for (i, (_, elt)) in element_pairs.iter().enumerate() {
        result.push_str(locator.slice(elt));
        if i < last_item_index {
            result.push_str(", ");
        }
    }

    result.push_str(closing_paren);
    result
}

/// An enumeration of the possible conclusions we could come to
/// regarding the ordering of the elements in a display of string literals
#[derive(Debug, is_macro::Is)]
pub(super) enum SortClassification<'a> {
    /// It's a display of string literals that is already sorted
    Sorted,
    /// It's an unsorted display of string literals,
    /// but we wouldn't be able to autofix it
    UnsortedButUnfixable,
    /// It's an unsorted display of string literals,
    /// and it's possible we could generate a fix for it
    UnsortedAndMaybeFixable { items: Vec<&'a str> },
    /// The display contains one or more items that are not string
    /// literals.
    NotAListOfStringLiterals,
}

impl<'a> SortClassification<'a> {
    pub(super) fn of_elements<F>(elements: &'a [ast::Expr], mut cmp_fn: F) -> Self
    where
        F: FnMut(&str, &str) -> Ordering,
    {
        let Some((first, rest @ [_, ..])) = elements.split_first() else {
            return Self::Sorted;
        };
        let Some(string_node) = first.as_string_literal_expr() else {
            return Self::NotAListOfStringLiterals;
        };
        let mut this = string_node.value.to_str();

        for expr in rest {
            let Some(string_node) = expr.as_string_literal_expr() else {
                return Self::NotAListOfStringLiterals;
            };
            let next = string_node.value.to_str();
            if cmp_fn(next, this).is_lt() {
                let mut items = Vec::with_capacity(elements.len());
                for expr in elements {
                    let Some(string_node) = expr.as_string_literal_expr() else {
                        return Self::NotAListOfStringLiterals;
                    };
                    if string_node.value.is_implicit_concatenated() {
                        return Self::UnsortedButUnfixable;
                    }
                    items.push(string_node.value.to_str());
                }
                return Self::UnsortedAndMaybeFixable { items };
            }
            this = next;
        }
        Self::Sorted
    }
}

// An instance of this struct encapsulates an analysis
/// of a multiline Python tuple/list that represents an
/// `__all__`/`__slots__`/etc. definition or augmentation.
pub(super) struct MultilineStringSequenceValue {
    items: Vec<StringSequenceItem>,
    range: TextRange,
    ends_with_trailing_comma: bool,
}

impl MultilineStringSequenceValue {
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    /// Analyse the source range for a multiline Python tuple/list that
    /// represents an `__all__`/`__slots__`/etc. definition or augmentation.
    /// Return `None` if the analysis fails for whatever reason.
    pub(super) fn from_source_range(
        range: TextRange,
        kind: &SequenceKind,
        locator: &Locator,
    ) -> Option<MultilineStringSequenceValue> {
        // Parse the multiline string sequence using the raw tokens.
        // See the docs for `collect_string_sequence_lines()` for why we have to
        // use the raw tokens, rather than just the AST, to do this parsing.
        //
        // Step (1). Start by collecting information on each line individually:
        let (lines, ends_with_trailing_comma) =
            collect_string_sequence_lines(range, kind, locator)?;

        // Step (2). Group lines together into sortable "items":
        //   - Any "item" contains a single element of the list/tuple
        //   - Assume that any comments on their own line are meant to be grouped
        //     with the element immediately below them: if the element moves,
        //     the comments above the element move with it.
        //   - The same goes for any comments on the same line as an element:
        //     if the element moves, the comment moves with it.
        let items = collect_string_sequence_items(lines, range, locator);

        Some(MultilineStringSequenceValue {
            items,
            range,
            ends_with_trailing_comma,
        })
    }

    /// Sort a multiline sequence of literal strings
    /// that is known to be unsorted.
    ///
    /// This function panics if it is called and `self.items`
    /// has length < 2. It's redundant to call this method in this case,
    /// since lists with < 2 items cannot be unsorted,
    /// so this is a logic error.
    pub(super) fn into_sorted_source_code<F>(
        mut self,
        cmp_fn: F,
        locator: &Locator,
        stylist: &Stylist,
    ) -> String
    where
        F: FnMut(&StringSequenceItem, &StringSequenceItem) -> Ordering,
    {
        let (first_item_start, last_item_end) = match self.items.as_slice() {
            [first_item, .., last_item] => (first_item.start(), last_item.end()),
            _ => panic!(
                "We shouldn't be attempting an autofix if a sequence has < 2 elements;
                a sequence with 1 or 0 elements cannot be unsorted."
            ),
        };

        // As well as the "items" in a multiline string sequence,
        // there is also a "prelude" and a "postlude":
        //  - Prelude == the region of source code from the opening parenthesis,
        //    up to the start of the first item in `__all__`/`__slots__`/etc.
        //  - Postlude == the region of source code from the end of the last
        //    item in `__all__`/`__slots__`/etc. up to and including the closing
        //    parenthesis.
        //
        // For example:
        //
        // ```python
        // __all__ = [  # comment0
        //   # comment1
        //   "first item",
        //   "last item"  # comment2
        //   # comment3
        // ]  # comment4
        // ```
        //
        // - The prelude in the above example is the source code region
        //   starting just before the opening `[` and ending just after `# comment0`.
        //   `comment0` here counts as part of the prelude because it is on
        //   the same line as the opening paren, and because we haven't encountered
        //   any elements of `__all__` yet, but `comment1` counts as part of the first item,
        //   as it's on its own line, and all comments on their own line are grouped
        //   with the next element below them to make "items",
        //   (an "item" being a region of source code that all moves as one unit
        //   when `__all__` is sorted).
        // - The postlude in the above example is the source code region starting
        //   just after `# comment2` and ending just after the closing paren.
        //   `# comment2` is part of the last item, as it's an inline comment on the
        //   same line as an element, but `# comment3` becomes part of the postlude
        //   because there are no items below it. `# comment4` is not part of the
        //   postlude: it's outside of the source-code range considered by this rule,
        //   and should therefore be untouched.
        //
        let newline = stylist.line_ending().as_str();
        let start_offset = self.start();
        let leading_indent = leading_indentation(locator.full_line(start_offset));
        let item_indent = format!("{}{}", leading_indent, stylist.indentation().as_str());

        let prelude =
            multiline_string_sequence_prelude(first_item_start, newline, start_offset, locator);
        let postlude = multiline_string_sequence_postlude(
            last_item_end,
            newline,
            leading_indent,
            &item_indent,
            self.end(),
            locator,
        );

        self.items.sort_by(cmp_fn);
        let joined_items = join_multiline_string_sequence_items(
            &self.items,
            locator,
            &item_indent,
            newline,
            self.ends_with_trailing_comma,
        );

        format!("{prelude}{joined_items}{postlude}")
    }
}

impl Ranged for MultilineStringSequenceValue {
    fn range(&self) -> TextRange {
        self.range
    }
}

/// Collect data on each line of a multiline string sequence.
/// Return `None` if the sequence appears to be invalid,
/// or if it's an edge case we don't support.
///
/// Why do we need to do this using the raw tokens,
/// when we already have the AST? The AST strips out
/// crucial information that we need to track here for
/// a multiline string sequence, such as:
/// - The value of comments
/// - The amount of whitespace between the end of a line
///   and an inline comment
/// - Whether or not the final item in the tuple/list has a
///   trailing comma
///
/// All of this information is necessary to have at a later
/// stage if we're to sort items without doing unnecessary
/// brutality to the comments and pre-existing style choices
/// in the original source code.
fn collect_string_sequence_lines(
    range: TextRange,
    kind: &SequenceKind,
    locator: &Locator,
) -> Option<(Vec<StringSequenceLine>, bool)> {
    // These first two variables are used for keeping track of state
    // regarding the entirety of the string sequence...
    let mut ends_with_trailing_comma = false;
    let mut lines = vec![];
    // ... all state regarding a single line of a string sequence
    // is encapsulated in this variable
    let mut line_state = LineState::default();

    // `lex_starts_at()` gives us absolute ranges rather than relative ranges,
    // but (surprisingly) we still need to pass in the slice of code we want it to lex,
    // rather than the whole source file:
    let mut token_iter =
        lexer::lex_starts_at(locator.slice(range), Mode::Expression, range.start());
    let (first_tok, _) = token_iter.next()?.ok()?;
    if TokenKind::from(&first_tok) != kind.opening_token_for_multiline_definition() {
        return None;
    }
    let expected_final_token = kind.closing_token_for_multiline_definition();

    for pair in token_iter {
        let (tok, subrange) = pair.ok()?;
        match tok {
            Tok::NonLogicalNewline => {
                lines.push(line_state.into_string_sequence_line());
                line_state = LineState::default();
            }
            Tok::Comment(_) => {
                line_state.visit_comment_token(subrange);
            }
            Tok::String { value, .. } => {
                line_state.visit_string_token(value, subrange);
                ends_with_trailing_comma = false;
            }
            Tok::Comma => {
                line_state.visit_comma_token(subrange);
                ends_with_trailing_comma = true;
            }
            tok if TokenKind::from(&tok) == expected_final_token => {
                lines.push(line_state.into_string_sequence_line());
                break;
            }
            _ => return None,
        }
    }
    Some((lines, ends_with_trailing_comma))
}

/// This struct is for keeping track of state
/// regarding a single line in a multiline string sequence
/// It is purely internal to `collect_string_sequence_lines()`,
/// and should not be used outside that function.
///
/// There are three possible kinds of line in a multiline
/// string sequence, and we don't know what kind of a line
/// we're in until all tokens in that line have been processed:
///
/// - A line with just a comment
///   (`StringSequenceLine::JustAComment)`)
/// - A line with one or more string items in it
///   (`StringSequenceLine::OneOrMoreItems`)
/// - An empty line (`StringSequenceLine::Empty`)
///
/// As we process the tokens in a single line,
/// this struct accumulates the necessary state for us
/// to be able to determine what kind of a line we're in.
/// Once the entire line has been processed,
/// `into_string_sequence_line()` is called, which consumes
/// `self` and produces the classification for the line.
#[derive(Debug, Default)]
struct LineState {
    first_item_in_line: Option<(String, TextRange)>,
    following_items_in_line: Vec<(String, TextRange)>,
    comment_range_start: Option<TextSize>,
    comment_in_line: Option<TextRange>,
}

impl LineState {
    fn visit_string_token(&mut self, token_value: String, token_range: TextRange) {
        if self.first_item_in_line.is_none() {
            self.first_item_in_line = Some((token_value, token_range));
        } else {
            self.following_items_in_line
                .push((token_value, token_range));
        }
        self.comment_range_start = Some(token_range.end());
    }

    fn visit_comma_token(&mut self, token_range: TextRange) {
        self.comment_range_start = Some(token_range.end());
    }

    /// If this is a comment on its own line,
    /// record the range of that comment.
    ///
    /// *If*, however, we've already seen a comma
    /// or a string in this line, that means that we're
    /// in a line with items. In that case, we want to
    /// record the range of the comment, *plus* the whitespace
    /// (if any) preceding the comment. This is so that we don't
    /// unnecessarily apply opinionated formatting changes
    /// where they might not be welcome.
    fn visit_comment_token(&mut self, token_range: TextRange) {
        self.comment_in_line = {
            if let Some(comment_range_start) = self.comment_range_start {
                Some(TextRange::new(comment_range_start, token_range.end()))
            } else {
                Some(token_range)
            }
        }
    }

    fn into_string_sequence_line(self) -> StringSequenceLine {
        if let Some(first_item) = self.first_item_in_line {
            StringSequenceLine::OneOrMoreItems(LineWithItems {
                first_item,
                following_items: self.following_items_in_line,
                trailing_comment_range: self.comment_in_line,
            })
        } else {
            self.comment_in_line
                .map_or(StringSequenceLine::Empty, |comment_range| {
                    StringSequenceLine::JustAComment(LineWithJustAComment(comment_range))
                })
        }
    }
}

/// Instances of this struct represent source-code lines in the middle
/// of multiline tuples/lists/sets where the line contains
/// 0 elements of the sequence, but the line does have a comment in it.
#[derive(Debug)]
struct LineWithJustAComment(TextRange);

/// Instances of this struct represent source-code lines in single-line
/// or multiline tuples/lists/sets where the line contains at least
/// 1 element of the sequence. The line may contain > 1 element of the
/// sequence, and may also have a trailing comment after the element(s).
#[derive(Debug)]
struct LineWithItems {
    // For elements in the list, we keep track of the value of the
    // value of the element as well as the source-code range of the element.
    // (We need to know the actual value so that we can sort the items.)
    first_item: (String, TextRange),
    following_items: Vec<(String, TextRange)>,
    // For comments, we only need to keep track of the source-code range.
    trailing_comment_range: Option<TextRange>,
}

impl LineWithItems {
    fn num_items(&self) -> usize {
        self.following_items.len() + 1
    }
}

/// An enumeration of the possible kinds of source-code lines
/// that can exist in a multiline string sequence:
///
/// - A line that has no string elements, but does have a comment.
/// - A line that has one or more string elements,
///   and may also have a trailing comment.
/// - An entirely empty line.
#[derive(Debug)]
enum StringSequenceLine {
    JustAComment(LineWithJustAComment),
    OneOrMoreItems(LineWithItems),
    Empty,
}

/// Given data on each line in a multiline string sequence,
/// group lines together into "items".
///
/// Each item contains exactly one string element,
/// but might contain multiple comments attached to that element
/// that must move with the element when the string sequence is sorted.
///
/// Note that any comments following the last item are discarded here,
/// but that doesn't matter: we add them back in `into_sorted_source_code()`
/// as part of the `postlude` (see comments in that function)
fn collect_string_sequence_items(
    lines: Vec<StringSequenceLine>,
    dunder_all_range: TextRange,
    locator: &Locator,
) -> Vec<StringSequenceItem> {
    let mut all_items = Vec::with_capacity(match lines.as_slice() {
        [StringSequenceLine::OneOrMoreItems(single)] => single.num_items(),
        _ => lines.len(),
    });
    let mut first_item_encountered = false;
    let mut preceding_comment_ranges = vec![];
    for line in lines {
        match line {
            StringSequenceLine::JustAComment(LineWithJustAComment(comment_range)) => {
                // Comments on the same line as the opening paren and before any elements
                // count as part of the "prelude"; these are not grouped into any item...
                if first_item_encountered
                    || locator.line_start(comment_range.start())
                        != locator.line_start(dunder_all_range.start())
                {
                    // ...but for all other comments that precede an element,
                    // group the comment with the element following that comment
                    // into an "item", so that the comment moves as one with the element
                    // when the list/tuple/set is sorted
                    preceding_comment_ranges.push(comment_range);
                }
            }
            StringSequenceLine::OneOrMoreItems(LineWithItems {
                first_item: (first_val, first_range),
                following_items,
                trailing_comment_range: comment_range,
            }) => {
                first_item_encountered = true;
                all_items.push(StringSequenceItem::new(
                    first_val,
                    std::mem::take(&mut preceding_comment_ranges),
                    first_range,
                    comment_range,
                ));
                for (value, range) in following_items {
                    all_items.push(StringSequenceItem::with_no_comments(value, range));
                }
            }
            StringSequenceLine::Empty => continue, // discard empty lines
        }
    }
    all_items
}

/// An instance of this struct represents a single element
/// from a multiline string sequence, *and* any comments that
/// are "attached" to it. The comments "attached" to the element
/// will move with the element when the tuple/list/set is sorted.
///
/// Comments on their own line immediately preceding the element will
/// always form a contiguous range with the range of the element itself;
/// however, inline comments won't necessary form a contiguous range.
/// Consider the following scenario, where both `# comment0` and `# comment1`
/// will move with the "a" element when the list is sorted:
///
/// ```python
/// __all__ = [
///     "b",
///     # comment0
///     "a", "c",  # comment1
/// ]
/// ```
///
/// The desired outcome here is:
///
/// ```python
/// __all__ = [
///     # comment0
///     "a",  # comment1
///     "b",
///     "c",
/// ]
/// ```
///
/// To achieve this, both `# comment0` and `# comment1`
/// are grouped into the `StringSequenceItem` instance
/// where the value is `"a"`, even though the source-code range
/// of `# comment1` does not form a contiguous range with the
/// source-code range of `"a"`.
#[derive(Debug)]
pub(super) struct StringSequenceItem {
    value: String,
    preceding_comment_ranges: Vec<TextRange>,
    element_range: TextRange,
    // total_range incorporates the ranges of preceding comments
    // (which must be contiguous with the element),
    // but doesn't incorporate any trailing comments
    // (which might be contiguous, but also might not be)
    total_range: TextRange,
    end_of_line_comments: Option<TextRange>,
}

impl StringSequenceItem {
    pub(super) fn value(&self) -> &str {
        &self.value
    }

    fn new(
        value: String,
        preceding_comment_ranges: Vec<TextRange>,
        element_range: TextRange,
        end_of_line_comments: Option<TextRange>,
    ) -> Self {
        let total_range = {
            if let Some(first_comment_range) = preceding_comment_ranges.first() {
                TextRange::new(first_comment_range.start(), element_range.end())
            } else {
                element_range
            }
        };
        Self {
            value,
            preceding_comment_ranges,
            element_range,
            total_range,
            end_of_line_comments,
        }
    }

    fn with_no_comments(value: String, element_range: TextRange) -> Self {
        Self::new(value, vec![], element_range, None)
    }
}

impl Ranged for StringSequenceItem {
    fn range(&self) -> TextRange {
        self.total_range
    }
}

/// Return a string representing the "prelude" for a
/// multiline string sequence.
///
/// See inline comments in
/// `MultilineStringSequenceValue::into_sorted_source_code()`
/// for a definition of the term "prelude" in this context.
fn multiline_string_sequence_prelude<'a>(
    first_item_start_offset: TextSize,
    newline: &str,
    dunder_all_offset: TextSize,
    locator: &'a Locator,
) -> Cow<'a, str> {
    let prelude_end = {
        let first_item_line_offset = locator.line_start(first_item_start_offset);
        if first_item_line_offset == locator.line_start(dunder_all_offset) {
            first_item_start_offset
        } else {
            first_item_line_offset
        }
    };
    let prelude = locator.slice(TextRange::new(dunder_all_offset, prelude_end));
    if prelude.ends_with(['\r', '\n']) {
        Cow::Borrowed(prelude)
    } else {
        Cow::Owned(format!("{}{}", prelude.trim_end(), newline))
    }
}

/// Join the elements and comments of a multiline string sequence
/// definition into a single string.
///
/// The resulting string does not include the "prelude" or
/// "postlude" of the tuple/set/list.
/// (See inline comments in
/// `MultilineStringSequence::into_sorted_source_code()` for
/// definitions of the terms "prelude" and "postlude" in this
/// context.)
fn join_multiline_string_sequence_items(
    sorted_items: &[StringSequenceItem],
    locator: &Locator,
    item_indent: &str,
    newline: &str,
    needs_trailing_comma: bool,
) -> String {
    let last_item_index = sorted_items.len() - 1;

    let mut new_dunder_all = String::new();
    for (i, item) in sorted_items.iter().enumerate() {
        let is_final_item = i == last_item_index;
        for comment_range in &item.preceding_comment_ranges {
            new_dunder_all.push_str(item_indent);
            new_dunder_all.push_str(locator.slice(comment_range));
            new_dunder_all.push_str(newline);
        }
        new_dunder_all.push_str(item_indent);
        new_dunder_all.push_str(locator.slice(item.element_range));
        if !is_final_item || needs_trailing_comma {
            new_dunder_all.push(',');
        }
        if let Some(trailing_comments) = item.end_of_line_comments {
            new_dunder_all.push_str(locator.slice(trailing_comments));
        }
        if !is_final_item {
            new_dunder_all.push_str(newline);
        }
    }
    new_dunder_all
}

/// Return a string representing the "postlude" for a
/// multiline string sequence.
///
/// See inline comments in
/// `MultilineStringSequence::into_sorted_source_code()`
/// for a definition of the term "postlude" in this context.
fn multiline_string_sequence_postlude<'a>(
    last_item_end_offset: TextSize,
    newline: &str,
    leading_indent: &str,
    item_indent: &str,
    dunder_all_range_end: TextSize,
    locator: &'a Locator,
) -> Cow<'a, str> {
    let postlude_start = {
        let last_item_line_offset = locator.line_end(last_item_end_offset);
        if last_item_line_offset == locator.line_end(dunder_all_range_end) {
            last_item_end_offset
        } else {
            last_item_line_offset
        }
    };
    let postlude = locator.slice(TextRange::new(postlude_start, dunder_all_range_end));

    // The rest of this function uses heuristics to
    // avoid very long indents for the closing paren
    // that don't match the style for the rest of the
    // fixed-up multiline string sequence.
    //
    // For example, we want to avoid something like this
    // (not uncommon in code that hasn't been
    // autoformatted)...
    //
    // ```python
    // __all__ = ["xxxxxx", "yyyyyy",
    //            "aaaaaa", "bbbbbb",
    //            ]
    // ```
    //
    // ...getting autofixed to this:
    //
    // ```python
    // __all__ = [
    //     "a",
    //     "b",
    //     "x",
    //     "y",
    //            ]
    // ```
    let newline_chars = ['\r', '\n'];
    if !postlude.starts_with(newline_chars) {
        return Cow::Borrowed(postlude);
    }
    if TextSize::of(leading_indentation(
        postlude.trim_start_matches(newline_chars),
    )) <= TextSize::of(item_indent)
    {
        return Cow::Borrowed(postlude);
    }
    let trimmed_postlude = postlude.trim_start();
    if trimmed_postlude.starts_with([']', ')']) {
        return Cow::Owned(format!("{newline}{leading_indent}{trimmed_postlude}"));
    }
    Cow::Borrowed(postlude)
}
