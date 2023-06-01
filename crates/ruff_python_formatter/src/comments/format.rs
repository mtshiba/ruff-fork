use crate::comments::SourceComment;
use crate::context::NodeLevel;
use crate::prelude::*;
use crate::trivia::{lines_after, lines_before};
use ruff_formatter::{format_args, write, SourceCode};
use ruff_python_ast::node::AnyNodeRef;
use ruff_python_ast::prelude::AstNode;
use ruff_text_size::{TextLen, TextRange, TextSize};

/// Formats the leading comments of a node.
pub(crate) fn leading_comments<T>(node: &T) -> FormatLeadingComments
where
    T: AstNode,
{
    FormatLeadingComments {
        node: node.as_any_node_ref(),
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct FormatLeadingComments<'a> {
    node: AnyNodeRef<'a>,
}

impl Format<PyFormatContext<'_>> for FormatLeadingComments<'_> {
    fn fmt(&self, f: &mut PyFormatter) -> FormatResult<()> {
        let comments = f.context().comments().clone();

        for comment in comments.leading_comments(self.node) {
            let slice = comment.slice();

            let lines_after_comment = lines_after(f.context().contents(), slice.end());
            write!(
                f,
                [format_comment(comment), empty_lines(lines_after_comment)]
            )?;

            comment.mark_formatted();
        }

        Ok(())
    }
}

/// Formats the trailing comments of `node`
pub(crate) fn trailing_comments<T>(node: &T) -> FormatTrailingComments
where
    T: AstNode,
{
    FormatTrailingComments {
        node: node.as_any_node_ref(),
    }
}

pub(crate) struct FormatTrailingComments<'a> {
    node: AnyNodeRef<'a>,
}

impl Format<PyFormatContext<'_>> for FormatTrailingComments<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        let comments = f.context().comments().clone();
        let mut has_empty_lines_before = false;

        for trailing in comments.trailing_comments(self.node) {
            let slice = trailing.slice();

            let lines_before_comment = lines_before(f.context().contents(), slice.start());
            has_empty_lines_before |= lines_before_comment > 0;

            if has_empty_lines_before {
                // A trailing comment at the end of a body or list
                // ```python
                // def test():
                //      pass
                //
                //      # Some comment
                // ```
                write!(
                    f,
                    [
                        line_suffix(&format_with(|f| {
                            write!(
                                f,
                                [empty_lines(lines_before_comment), format_comment(trailing)]
                            )
                        })),
                        expand_parent()
                    ]
                )?;
            } else {
                write!(
                    f,
                    [
                        line_suffix(&format_args![space(), space(), format_comment(trailing)]),
                        expand_parent()
                    ]
                )?;
            }

            trailing.mark_formatted();
        }

        Ok(())
    }
}

/// Formats the dangling comments of `node`.
pub(crate) fn dangling_comments<T>(node: &T) -> FormatDanglingComments
where
    T: AstNode,
{
    FormatDanglingComments {
        node: node.as_any_node_ref(),
    }
}

pub(crate) struct FormatDanglingComments<'a> {
    node: AnyNodeRef<'a>,
}

impl Format<PyFormatContext<'_>> for FormatDanglingComments<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext>) -> FormatResult<()> {
        let comments = f.context().comments().clone();

        let dangling_comments = comments.dangling_comments(self.node);

        let mut first = true;
        for comment in dangling_comments {
            if first && comment.position().is_end_of_line() {
                write!(f, [space(), space()])?;
            }

            write!(
                f,
                [
                    format_comment(comment),
                    empty_lines(lines_after(f.context().contents(), comment.slice().end()))
                ]
            )?;

            comment.mark_formatted();

            first = false;
        }

        Ok(())
    }
}

/// Formats the content of the passed comment.
///
/// * Adds a whitespace between `#` and the comment text except if the first character is a `#`, `:`, `'`, or `!`
/// * Replaces non breaking whitespaces with regular whitespaces except if in front of a `types:` comment
const fn format_comment(comment: &SourceComment) -> FormatComment {
    FormatComment { comment }
}

struct FormatComment<'a> {
    comment: &'a SourceComment,
}

impl Format<PyFormatContext<'_>> for FormatComment<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        let slice = self.comment.slice();
        let comment_text = slice.text(SourceCode::new(f.context().contents()));

        let (content, mut start_offset) = comment_text
            .strip_prefix('#')
            .map_or((comment_text, TextSize::new(0)), |rest| {
                (rest, '#'.text_len())
            });

        write!(f, [source_position(slice.start()), text("#")])?;

        // Starts with a non breaking space
        if content.starts_with('\u{A0}') && !content.trim_start().starts_with("type:") {
            // Replace non-breaking space with a space (if not followed by a normal space)
            start_offset += '\u{A0}'.text_len();
        }

        // Add a space between the `#` and the text if the source contains none.
        if !content.is_empty() && !content.starts_with([' ', '!', ':', '#', '\'']) {
            write!(f, [space()])?;
        }

        let start = slice.start() + start_offset;
        let trimmed = content.trim_end();
        let end = slice.range().end() - (content.text_len() - trimmed.text_len());

        write!(
            f,
            [
                source_text_slice(TextRange::new(start, end), ContainsNewlines::No),
                source_position(slice.end())
            ]
        )
    }
}

// Helper that inserts the appropriate number of empty lines before a comment, depending on the node level.
// Top level: Up to two empty lines
// parenthesized: A single empty line
// other: Up to a single empty line
const fn empty_lines(lines: u32) -> FormatEmptyLines {
    FormatEmptyLines { lines }
}

#[derive(Copy, Clone, Debug)]
struct FormatEmptyLines {
    lines: u32,
}

impl Format<PyFormatContext<'_>> for FormatEmptyLines {
    fn fmt(&self, f: &mut Formatter<PyFormatContext>) -> FormatResult<()> {
        match f.context().node_level() {
            NodeLevel::TopLevel => match self.lines {
                0 | 1 => write!(f, [hard_line_break()]),
                2 => write!(f, [empty_line()]),
                _ => write!(f, [empty_line(), empty_line()]),
            },

            NodeLevel::Statement => match self.lines {
                0 | 1 => write!(f, [hard_line_break()]),
                _ => write!(f, [empty_line()]),
            },

            // Remove all whitespace in parenthesized expressions
            NodeLevel::Parenthesized => write!(f, [hard_line_break()]),
        }
    }
}
