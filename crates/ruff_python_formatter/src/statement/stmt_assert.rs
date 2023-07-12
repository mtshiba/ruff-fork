use crate::builders::parenthesize_if_expands;
use crate::expression::parentheses::Parenthesize;
use crate::{AsFormat, FormatNodeRule, PyFormatter};
use ruff_formatter::prelude::{format_args, group, space, text};
use ruff_formatter::{write, Buffer, FormatResult};
use rustpython_parser::ast::StmtAssert;

#[derive(Default)]
pub struct FormatStmtAssert;

impl FormatNodeRule<StmtAssert> for FormatStmtAssert {
    fn fmt_fields(&self, item: &StmtAssert, f: &mut PyFormatter) -> FormatResult<()> {
        let StmtAssert {
            range: _,
            test,
            msg,
        } = item;

        write!(f, [text("assert"), space()])?;

        if let Some(msg) = msg {
            write!(
                f,
                [group(&format_args![
                    test.format().with_options(Parenthesize::IfBreaks),
                    text(","),
                    space(),
                    // `msg` gets parentheses if expanded so we don't need any beyond that.
                    parenthesize_if_expands(&msg.format().with_options(Parenthesize::Never))
                ])]
            )?;
        } else {
            write!(f, [test.format().with_options(Parenthesize::IfBreaks)])?;
        }

        Ok(())
    }
}
