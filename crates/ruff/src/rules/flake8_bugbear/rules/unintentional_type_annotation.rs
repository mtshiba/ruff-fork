use rustpython_parser::ast::{self, Expr, ExprKind, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for the unintentional use of type annotations.
///
/// ## Why is this bad?
/// The use of a colon (`:`) in lieu of an assignment (`=`) can be syntactically valid, but
/// is almost certainly a mistake when used in a subscript or attribute assignment.
///
/// ## Example
/// ```python
/// a["b"]: 1
/// ```
///
/// Use instead:
/// ```python
/// a["b"] = 1
/// ```
#[violation]
pub struct UnintentionalTypeAnnotation;

impl Violation for UnintentionalTypeAnnotation {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!(
            "Possible unintentional type annotation (using `:`). Did you mean to assign (using `=`)?"
        )
    }
}

/// B032
pub(crate) fn unintentional_type_annotation(
    checker: &mut Checker,
    target: &Expr,
    value: Option<&Expr>,
    stmt: &Stmt,
) {
    if value.is_some() {
        return;
    }
    match &target.node {
        ExprKind::Subscript(ast::ExprSubscript { value, .. }) => {
            if matches!(&value.node, ExprKind::Name(_)) {
                checker
                    .diagnostics
                    .push(Diagnostic::new(UnintentionalTypeAnnotation, stmt.range()));
            }
        }
        ExprKind::Attribute(ast::ExprAttribute { value, .. }) => {
            if let ExprKind::Name(ast::ExprName { id, .. }) = &value.node {
                if id != "self" {
                    checker
                        .diagnostics
                        .push(Diagnostic::new(UnintentionalTypeAnnotation, stmt.range()));
                }
            }
        }
        _ => {}
    };
}
