use ruff_formatter::prelude::{format_with, space, text};
use ruff_formatter::{format_args, write, Buffer, Format, FormatResult};
use ruff_python_ast::PatternMatchSequence;

use crate::builders::PyFormatterExtensions;
use crate::expression::parentheses::{empty_parenthesized, parenthesized};
use crate::AsFormat;
use crate::{FormatNodeRule, PyFormatter};

#[derive(Default)]
pub struct FormatPatternMatchSequence;

#[derive(Debug)]
enum SequenceType {
    Tuple,
    TupleWithoutParentheses,
    List,
}

impl FormatNodeRule<PatternMatchSequence> for FormatPatternMatchSequence {
    fn fmt_fields(&self, item: &PatternMatchSequence, f: &mut PyFormatter) -> FormatResult<()> {
        let PatternMatchSequence { patterns, range } = item;
        let sequence_type = match &f.context().source()[*range].chars().next() {
            Some('(') => SequenceType::Tuple,
            Some('[') => SequenceType::List,
            _ => SequenceType::TupleWithoutParentheses,
        };
        let comments = f.context().comments().clone();
        let dangling = comments.dangling(item);
        if patterns.is_empty() {
            return match sequence_type {
                SequenceType::Tuple => empty_parenthesized("(", dangling, ")").fmt(f),
                SequenceType::List => empty_parenthesized("[", dangling, "]").fmt(f),
                SequenceType::TupleWithoutParentheses => {
                    unreachable!("If empty, it should be either tuple or list")
                }
            };
        }

        match sequence_type {
            SequenceType::Tuple => {
                let items = format_with(|f| {
                    f.join_comma_separated(range.end())
                        .nodes(patterns.iter())
                        .finish()
                });
                parenthesized("(", &items, ")")
                    .with_dangling_comments(dangling)
                    .fmt(f)
            }
            SequenceType::List => {
                let items = format_with(|f| {
                    f.join_comma_separated(range.end())
                        .nodes(patterns.iter())
                        .finish()
                });
                parenthesized("[", &items, "]")
                    .with_dangling_comments(dangling)
                    .fmt(f)
            }
            SequenceType::TupleWithoutParentheses => {
                let items = format_with(|f| {
                    f.join_with(&format_args![text(","), space()])
                        .entries(patterns.iter().map(AsFormat::format))
                        .finish()
                });
                write!(f, [items])
            }
        }
    }
}
