use rustpython_parser::ast::{self, Expr, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::helpers::RaiseStatementVisitor;
use ruff_python_ast::statement_visitor::StatementVisitor;
use ruff_python_stdlib::str::is_cased_lowercase;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for `raise` statements without a `from` clause inside an `except` clause.
///
/// ## Why is this bad?
/// Raising exceptions without a `from` clause inside an `except` clause makes it
/// difficult to distinguish between exceptions raised by the `raise` statement
/// and exceptions raised by the `except` clause.
///
/// Instead, use the `from` clause to distinguish between the two (for example,
/// `raise ... from exc` or `raise ... from None`).
///
/// ## Example
/// ```python
/// try:
///     ...
/// except FileNotFoundError:
///     if condition:
///         raise RuntimeError("...")
///     else:
///         raise UserWarning("...")
/// ```
///
/// Use instead:
/// ```python
/// try:
///     ...
/// except FileNotFoundError as exc:
///     if condition:
///         raise RuntimeError("...") from None
///     else:
///         raise UserWarning("...") from exc
/// ```
///
/// ## References
/// - [Python documentation: `raise` statement](https://docs.python.org/3/reference/simple_stmts.html#the-raise-statement)
#[violation]
pub struct RaiseWithoutFromInsideExcept;

impl Violation for RaiseWithoutFromInsideExcept {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!(
            "Within an `except` clause, raise exceptions with `raise ... from err` or `raise ... \
             from None` to distinguish them from errors in exception handling"
        )
    }
}

/// B904
pub(crate) fn raise_without_from_inside_except(checker: &mut Checker, body: &[Stmt]) {
    let raises = {
        let mut visitor = RaiseStatementVisitor::default();
        visitor.visit_body(body);
        visitor.raises
    };

    for (range, exc, cause) in raises {
        if cause.is_none() {
            if let Some(exc) = exc {
                match exc {
                    Expr::Name(ast::ExprName { id, .. }) if is_cased_lowercase(id) => {}
                    _ => {
                        checker
                            .diagnostics
                            .push(Diagnostic::new(RaiseWithoutFromInsideExcept, range));
                    }
                }
            }
        }
    }
}
