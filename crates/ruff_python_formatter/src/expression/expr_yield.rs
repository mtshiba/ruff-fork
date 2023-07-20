use crate::context::PyFormatContext;
use crate::expression::maybe_parenthesize_expression;
use crate::expression::parentheses::{NeedsParentheses, OptionalParentheses, Parenthesize};
use crate::{FormatNodeRule, PyFormatter};
use ruff_formatter::prelude::{space, text};
use ruff_formatter::{write, Buffer, FormatResult};
use ruff_python_ast::node::AnyNodeRef;
use rustpython_parser::ast::ExprYield;

#[derive(Default)]
pub struct FormatExprYield;

impl FormatNodeRule<ExprYield> for FormatExprYield {
    fn fmt_fields(&self, item: &ExprYield, f: &mut PyFormatter) -> FormatResult<()> {
        let ExprYield { range: _, value } = item;

        if let Some(val) = value {
            write!(
                f,
                [
                    &text("yield"),
                    space(),
                    maybe_parenthesize_expression(val, item, Parenthesize::IfRequired)
                ]
            )?;
        } else {
            write!(f, [&text("yield")])?;
        }
        Ok(())
    }
}

impl NeedsParentheses for ExprYield {
    fn needs_parentheses(
        &self,
        parent: AnyNodeRef,
        _context: &PyFormatContext,
    ) -> OptionalParentheses {
        if parent.is_stmt_assign() || parent.is_stmt_ann_assign() || parent.is_stmt_aug_assign() {
            OptionalParentheses::Multiline
        } else {
            OptionalParentheses::Always
        }
    }
}
