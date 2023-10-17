use ast::{Arguments, Stmt};
use ruff_python_ast::{self as ast, Expr, StmtFor};

use ruff_diagnostics::{AlwaysFixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::visitor;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::TextRange;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for uses of `dict.items()` and accessing the value by index lookup.
///
/// ## Why is this bad?
/// The value is already accessible by the 2nd variable from `dict.items()`.
///
/// ## Example
/// ```python
/// FRUITS = {"apple": 1, "orange": 10, "berry": 22}
///
/// for fruit_name, fruit_count in FRUITS.items():
///     print(FRUITS[fruit_name])
/// ```
///
/// Use instead:
/// ```python
/// FRUITS = {"apple": 1, "orange": 10, "berry": 22}
///
/// for fruit_name, fruit_count in FRUITS.items():
///     print(fruit_count)
/// ```
#[violation]
pub struct UnnecessaryDictIndexLookup;

impl AlwaysFixableViolation for UnnecessaryDictIndexLookup {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Unnecessary dict index lookup")
    }

    fn fix_title(&self) -> String {
        format!("Remove unnecessary dict index lookup")
    }
}

struct SubscriptVisitor<'a> {
    dict_name: &'a str,
    index_name: &'a str,
    diagnostic_ranges: Vec<TextRange>,
}

impl<'a> SubscriptVisitor<'a> {
    fn new(dict_name: &'a str, index_name: &'a str) -> Self {
        Self {
            dict_name,
            index_name,
            diagnostic_ranges: Vec::new(),
        }
    }
}

impl<'a> Visitor<'_> for SubscriptVisitor<'a> {
    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Subscript(ast::ExprSubscript {
                value,
                slice,
                range,
                ..
            }) => {
                if let Expr::Name(ast::ExprName { id, .. }) = value.as_ref() {
                    if id == self.dict_name {
                        if let Expr::Name(ast::ExprName { id, .. }) = slice.as_ref() {
                            if id == self.index_name {
                                self.diagnostic_ranges.push(*range);
                            }
                        }
                    }
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(ast::StmtAssign { value, .. }) => {
                self.visit_expr(value);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

/// PLR1733
pub(crate) fn unnecessary_dict_index_lookup(checker: &mut Checker, stmt_for: &StmtFor) {
    let Expr::Call(ast::ExprCall {
        func,
        arguments: Arguments { args, .. },
        ..
    }) = stmt_for.iter.as_ref()
    else {
        return;
    };
    if !args.is_empty() {
        return;
    }
    let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = func.as_ref() else {
        return;
    };
    if attr != "items" {
        return;
    }

    let Expr::Name(ast::ExprName { id: dict_name, .. }) = value.as_ref() else {
        return;
    };

    let Expr::Tuple(ast::ExprTuple { elts, .. }) = stmt_for.target.as_ref() else {
        return;
    };
    let [index, value] = elts.as_slice() else {
        return;
    };

    // Grab the variable names
    let Expr::Name(ast::ExprName { id: index_name, .. }) = index else {
        return;
    };

    let Expr::Name(ast::ExprName { id: value_name, .. }) = value else {
        return;
    };

    // If either of the variable names are intentionally ignored by naming them `_`, then don't emit
    if index_name == "_" || value_name == "_" {
        return;
    }

    let mut visitor = SubscriptVisitor::new(dict_name, index_name);

    visitor.visit_body(&stmt_for.body);
    visitor.visit_body(&stmt_for.orelse);

    for range in visitor.diagnostic_ranges {
        let mut diagnostic = Diagnostic::new(UnnecessaryDictIndexLookup, range);

        diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
            value_name.clone(),
            range,
        )));

        checker.diagnostics.push(diagnostic);
    }
}
