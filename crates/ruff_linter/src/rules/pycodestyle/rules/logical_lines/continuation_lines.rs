use std::cmp::max;
use std::iter::zip;

use super::{LogicalLine, LogicalLineToken};
use crate::checkers::logical_lines::LogicalLinesContext;
use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_index::Indexer;
use ruff_python_parser::TokenKind;
use ruff_source_file::Locator;
use ruff_text_size::{Ranged, TextRange, TextSize};

/// ## What it does
/// Checks for continuation lines without enough indentation.
///
/// ## Why is this bad?
/// This makes distinguishing continuation lines more difficult.
///
/// ## Example
/// ```python
/// print("Python", (
/// "Rules"))
/// ```
///
/// Use instead:
/// ```python
/// print("Python", (
///     "Rules"))
/// ```
///
/// [PEP 8]: https://www.python.org/dev/peps/pep-0008/#indentation
#[violation]
pub struct MissingOrOutdentedIndentation;

impl Violation for MissingOrOutdentedIndentation {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Continuation line missing indentation or outdented.")
    }
}

#[derive(Debug)]
struct TokenInfo {
    start_physical_line_idx: usize,
    end_physical_line_idx: usize,
    token_start_within_physical_line: i64,
    token_end_within_physical_line: i64,
}

/// Compute the `TokenInfo` of each token.
fn get_token_infos<'a>(
    logical_line: &LogicalLine,
    locator: &'a Locator,
    indexer: &'a Indexer,
) -> Vec<TokenInfo> {
    let mut token_infos = Vec::with_capacity(logical_line.tokens().len());
    let mut current_line_idx: usize = 0;
    // The first physical line occupied by the token, since a token can span multiple physical lines.
    let mut first_physical_line_start: usize;
    let mut next_continuation;
    if let Some(first_token) = logical_line.first_token() {
        first_physical_line_start = first_token.range.start().into();
        next_continuation = continuation_line_end(first_token, locator, indexer);
    } else {
        return token_infos;
    };

    let mut current_physical_line_start: usize;
    let mut prev_token: Option<&LogicalLineToken> = None;
    for token in logical_line.tokens() {
        let mut start_physical_line_idx = current_line_idx;
        current_physical_line_start = first_physical_line_start;

        // Check for escaped ('\') continuation lines between the previous and current tokens.
        if let Some(prev_token) = prev_token {
            if next_continuation.is_some() && token.start() >= next_continuation.unwrap() {
                next_continuation = continuation_line_end(token, locator, indexer);

                let trivia =
                    locator.slice(TextRange::new(prev_token.range.end(), token.range.start()));
                for (index, _text) in trivia.match_indices('\n') {
                    start_physical_line_idx += 1;
                    current_line_idx = start_physical_line_idx;
                    first_physical_line_start = usize::from(prev_token.range.end()) + index + 1;
                    current_physical_line_start = first_physical_line_start;
                }
            }
        }

        if matches!(
            token.kind,
            TokenKind::String
                | TokenKind::FStringStart
                | TokenKind::FStringMiddle
                | TokenKind::FStringEnd
        ) {
            // Look for newlines within strings.
            let trivia = locator.slice(token.range);
            for (index, _text) in trivia.match_indices('\n') {
                current_line_idx += 1;
                current_physical_line_start = usize::from(token.range.start()) + index + 1;
            }
        }

        token_infos.push(TokenInfo {
            start_physical_line_idx,
            end_physical_line_idx: current_line_idx,
            token_start_within_physical_line: i64::try_from(
                usize::from(token.range.start()) - first_physical_line_start,
            )
            .expect("Lines are expected to be relatively short."),
            token_end_within_physical_line: i64::try_from(
                usize::from(token.range.end()) - current_physical_line_start,
            )
            .expect("Lines are expected to be relatively short."),
        });

        if matches!(
            token.kind,
            TokenKind::NonLogicalNewline | TokenKind::Newline
        ) {
            current_line_idx += 1;
            first_physical_line_start = token.range.end().into();
        } else {
            first_physical_line_start = current_physical_line_start;
        }
        prev_token = Some(token);
    }

    token_infos
}

fn continuation_line_end(
    token: &LogicalLineToken,
    locator: &Locator,
    indexer: &Indexer,
) -> Option<TextSize> {
    let line_start = locator.line_start(token.start());
    let continuation_lines = indexer.continuation_line_starts();
    let continuation_line_index = continuation_lines
        .binary_search(&line_start)
        .unwrap_or_else(|err_index| err_index);
    let continuation_line_start = continuation_lines.get(continuation_line_index)?;
    Some(locator.full_line_end(*continuation_line_start))
}

/// Return the amount of indentation of the given line.
/// Tabs are expanded to the next multiple of 8.
fn expand_indent(line: &str) -> i64 {
    if !line.contains('\t') {
        // If there are no tabs in the line, return the leading space count
        return i64::try_from(line.len() - line.trim_start().len())
            .expect("Line length to be relatively small.");
    }
    let mut indent = 0;

    for ch in line.chars() {
        if ch == '\t' {
            indent = indent / 8 * 8 + 8;
        } else if ch == ' ' {
            indent += 1;
        } else {
            break;
        }
    }

    indent
}

fn calculate_max_depth(logical_line: &LogicalLine) -> usize {
    let mut depth = 0;
    let mut max_depth = 0;
    for token in logical_line.tokens() {
        match token.kind {
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => {
                depth += 1;
                max_depth = max(max_depth, depth);
            }
            TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => depth -= 1,
            _ => continue,
        }
    }
    max_depth
}

fn valid_hang(hang: i64, indent_size: i64, indent_char: char) -> bool {
    hang == indent_size || (indent_char == '\t' && hang == 2 * indent_size)
}

/// E122
pub(crate) fn continuation_lines(
    logical_line: &LogicalLine,
    indent_char: char,
    indent_size: usize,
    locator: &Locator,
    indexer: &Indexer,
    context: &mut LogicalLinesContext,
) {
    // The pycodestyle implementation makes use of negative values,
    // converting the indent_size type at the start avoids converting it multiple times later.
    let indent_size = i64::try_from(indent_size).expect("Indent size to be relatively small.");
    let token_infos = get_token_infos(logical_line, locator, indexer);
    let nb_physical_lines = if let Some(last_token_info) = token_infos.last() {
        1 + last_token_info.start_physical_line_idx
    } else {
        1
    };

    if nb_physical_lines == 1 {
        return;
    }

    // Indent of the first physical line.
    let start_indent_level = expand_indent(
        locator.line(
            logical_line
                .first_token()
                .expect("Would have returned earlier if the logical line was empty")
                .start(),
        ),
    );

    // Here "row" is the physical line index (within the logical line).
    let mut row = 0;
    let mut depth = 0;
    let max_depth = calculate_max_depth(logical_line);
    // Brackets opened on a line.
    let mut brackets_opened = 0u32;
    // In fstring
    let mut fstrings_opened = 0u32;
    // Relative indents of physical lines.
    let mut rel_indent: Vec<i64> = vec![0; nb_physical_lines];
    // For each depth, collect a list of opening rows.
    let mut open_rows: Vec<Vec<usize>> = Vec::with_capacity(max_depth + 1);
    open_rows.push(vec![0]);
    // For each depth, record the hanging indentation.
    let mut hangs: Vec<Option<i64>> = Vec::with_capacity(max_depth + 1);
    hangs.push(None);
    let mut hang: i64 = 0;
    // Visual indents
    let mut last_indent = start_indent_level;
    let mut last_token_multiline = false;
    // For each depth, record the visual indent column.
    let mut indent = Vec::with_capacity(max_depth + 1);
    indent.push(start_indent_level);

    for (token, token_info) in zip(logical_line.tokens(), &token_infos) {
        let mut is_newline = row < token_info.start_physical_line_idx;
        if is_newline {
            row = token_info.start_physical_line_idx;
            brackets_opened = 0;
            is_newline = !last_token_multiline
                && !matches!(
                    token.kind,
                    TokenKind::NonLogicalNewline | TokenKind::Newline
                );
        }

        let is_closing_bracket = matches!(
            token.kind,
            TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace
        );

        // This is the beginning of a continuation line.
        if is_newline {
            last_indent = token_info.token_start_within_physical_line;

            // Record the initial indent.
            let indent_range = TextRange::new(locator.line_start(token.start()), token.start());
            rel_indent[row] = expand_indent(locator.slice(indent_range)) - start_indent_level;

            // Is the indent relative to an opening bracket line ?
            for open_row in open_rows[depth].iter().rev() {
                hang = rel_indent[row] - rel_indent[*open_row];
                if valid_hang(hang, indent_size, indent_char) {
                    break;
                }
            }

            let is_visual_indent_violation =
                token_info.token_start_within_physical_line < indent[depth];
            // E122 is triggered by the following:
            // 1. There is no visual indent violation (this is a different rule in pycodestyle)
            // 2. The relative hang is less than or equal to zero.
            // 3. Unless this is a closing bracket, in which case it can be zero.
            if !is_visual_indent_violation && (hang < 0 || (!is_closing_bracket && hang == 0)) {
                // E122.
                let diagnostic = Diagnostic::new(MissingOrOutdentedIndentation, indent_range);
                context.push_diagnostic(diagnostic);
            }
        }

        // Look for visual indenting.
        if brackets_opened != 0
            && !matches!(
                token.kind,
                TokenKind::Newline | TokenKind::NonLogicalNewline | TokenKind::Comment
            )
            && indent[depth] == 0
        {
            indent[depth] = token_info.token_start_within_physical_line;
        }

        if matches!(token.kind, TokenKind::Colon)
            && locator.full_lines(token.range)[usize::try_from(
                token_info.token_end_within_physical_line,
            )
            .expect("Line to be relatively short.")..]
                .trim()
                .is_empty()
        {
            open_rows[depth].push(row);
        }

        let is_opening_bracket = matches!(
            token.kind,
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace
        );

        if matches!(token.kind, TokenKind::FStringStart) {
            fstrings_opened += 1;
        } else if matches!(token.kind, TokenKind::FStringEnd) {
            fstrings_opened -= 1;
        }

        // Keep track of bracket depth.
        if fstrings_opened == 0 {
            if is_opening_bracket {
                depth += 1;
                indent.push(0);
                hangs.push(None);
                if open_rows.len() == depth {
                    open_rows.push(Vec::new());
                }
                open_rows[depth].push(row);
                brackets_opened += 1;
            } else if is_closing_bracket && depth > 0 {
                // Parent indents should not be more than this one.
                let prev_indent = if let Some(i) = indent.pop() {
                    if i > 0 {
                        i
                    } else {
                        last_indent
                    }
                } else {
                    last_indent
                };
                hangs.pop();
                for ind in indent.iter_mut().take(depth) {
                    if *ind > prev_indent {
                        *ind = 0;
                    }
                }
                open_rows.truncate(depth + 1);
                depth -= 1;
                brackets_opened = brackets_opened.saturating_sub(1);
            }
        }

        last_token_multiline =
            token_info.start_physical_line_idx != token_info.end_physical_line_idx;
        if last_token_multiline {
            rel_indent[token_info.end_physical_line_idx] = rel_indent[row];
        }
    }
}
