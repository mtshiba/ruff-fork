use ruff_formatter::{format_args, write, Buffer, FormatResult};
use ruff_python_ast::node::AnyNodeRef;
use ruff_python_ast::ExprSetComp;

use crate::context::PyFormatContext;
use crate::expression::parentheses::{
    parenthesized_with_head_comments, NeedsParentheses, OptionalParentheses,
};
use crate::prelude::*;
use crate::AsFormat;
use crate::{FormatNodeRule, PyFormatter};

#[derive(Default)]
pub struct FormatExprSetComp;

impl FormatNodeRule<ExprSetComp> for FormatExprSetComp {
    fn fmt_fields(&self, item: &ExprSetComp, f: &mut PyFormatter) -> FormatResult<()> {
        let ExprSetComp {
            range: _,
            elt,
            generators,
        } = item;

        let joined = format_with(|f| {
            f.join_with(soft_line_break_or_space())
                .entries(generators.iter().formatted())
                .finish()
        });

        let comments = f.context().comments().clone();
        let dangling = comments.dangling_comments(item);

        write!(
            f,
            [parenthesized_with_head_comments(
                "{",
                dangling,
                &group(&format_args!(
                    group(&elt.format()),
                    soft_line_break_or_space(),
                    group(&joined)
                )),
                "}"
            )]
        )
    }

    fn fmt_dangling_comments(&self, _node: &ExprSetComp, _f: &mut PyFormatter) -> FormatResult<()> {
        // Handled as part of `fmt_fields`
        Ok(())
    }
}

impl NeedsParentheses for ExprSetComp {
    fn needs_parentheses(
        &self,
        _parent: AnyNodeRef,
        _context: &PyFormatContext,
    ) -> OptionalParentheses {
        OptionalParentheses::Never
    }
}
