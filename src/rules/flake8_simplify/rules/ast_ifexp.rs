use rustpython_ast::{Constant, Expr, ExprContext, ExprKind, Unaryop};

use crate::ast::helpers::{create_expr, unparse_expr};
use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::fix::Fix;
use crate::registry::Diagnostic;
use crate::violations;

/// SIM210
pub fn explicit_true_false_in_ifexpr(
    checker: &mut Checker,
    expr: &Expr,
    test: &Expr,
    body: &Expr,
    orelse: &Expr,
) {
    let ExprKind::Constant { value, .. } = &body.node else {
        return;
    };
    if !matches!(value, Constant::Bool(true)) {
        return;
    }
    let ExprKind::Constant { value, .. } = &orelse.node else {
        return;
    };
    if !matches!(value, Constant::Bool(false)) {
        return;
    }

    let mut diagnostic = Diagnostic::new(
        violations::IfExprWithTrueFalse {
            expr: unparse_expr(test, checker.stylist),
        },
        Range::from_located(expr),
    );
    if checker.patch(diagnostic.kind.rule()) {
        let mut fixit = true;
        let bool_expr;
        if let ExprKind::Compare { .. } = test.node {
            bool_expr = test.clone();
        } else {
            fixit = checker.is_builtin("bool");
            bool_expr = create_expr(ExprKind::Call {
                func: Box::new(create_expr(ExprKind::Name {
                    id: "bool".to_string(),
                    ctx: ExprContext::Load,
                })),
                args: vec![test.clone()],
                keywords: vec![],
            });
        };

        if fixit {
            diagnostic.amend(Fix::replacement(
                unparse_expr(&bool_expr, checker.stylist),
                expr.location,
                expr.end_location.unwrap(),
            ));
        }
    }
    checker.diagnostics.push(diagnostic);
}

/// SIM211
pub fn explicit_false_true_in_ifexpr(
    checker: &mut Checker,
    expr: &Expr,
    test: &Expr,
    body: &Expr,
    orelse: &Expr,
) {
    let ExprKind::Constant { value, .. } = &body.node else {
        return;
    };
    if !matches!(value, Constant::Bool(false)) {
        return;
    }
    let ExprKind::Constant { value, .. } = &orelse.node else {
        return;
    };
    if !matches!(value, Constant::Bool(true)) {
        return;
    }

    let mut diagnostic = Diagnostic::new(
        violations::IfExprWithFalseTrue {
            expr: unparse_expr(test, checker.stylist),
        },
        Range::from_located(expr),
    );
    if checker.patch(diagnostic.kind.rule()) {
        diagnostic.amend(Fix::replacement(
            unparse_expr(
                &create_expr(ExprKind::UnaryOp {
                    op: Unaryop::Not,
                    operand: Box::new(create_expr(test.node.clone())),
                }),
                checker.stylist,
            ),
            expr.location,
            expr.end_location.unwrap(),
        ));
    }
    checker.diagnostics.push(diagnostic);
}

/// SIM212
pub fn twisted_arms_in_ifexpr(
    checker: &mut Checker,
    expr: &Expr,
    test: &Expr,
    body: &Expr,
    orelse: &Expr,
) {
    let ExprKind::UnaryOp { op, operand: test_operand } = &test.node else {
        return;
    };
    if !matches!(op, Unaryop::Not) {
        return;
    }

    // Check if the test operand and else branch use the same variable.
    let ExprKind::Name { id: test_id, .. } = &test_operand.node else {
        return;
    };
    let ExprKind::Name {id: orelse_id, ..} = &orelse.node else {
        return;
    };
    if !test_id.eq(orelse_id) {
        return;
    }

    let mut diagnostic = Diagnostic::new(
        violations::IfExprWithTwistedArms {
            expr_body: unparse_expr(body, checker.stylist),
            expr_else: unparse_expr(orelse, checker.stylist),
        },
        Range::from_located(expr),
    );
    if checker.patch(diagnostic.kind.rule()) {
        diagnostic.amend(Fix::replacement(
            unparse_expr(
                &create_expr(ExprKind::IfExp {
                    test: Box::new(create_expr(orelse.node.clone())),
                    body: Box::new(create_expr(orelse.node.clone())),
                    orelse: Box::new(create_expr(body.node.clone())),
                }),
                checker.stylist,
            ),
            expr.location,
            expr.end_location.unwrap(),
        ));
    }
    checker.diagnostics.push(diagnostic);
}
