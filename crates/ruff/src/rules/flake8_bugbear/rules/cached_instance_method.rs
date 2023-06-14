use rustpython_parser::ast::{self, Decorator, Expr, Ranged};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_semantic::SemanticModel;

use crate::checkers::ast::Checker;

#[violation]
pub struct CachedInstanceMethod;

impl Violation for CachedInstanceMethod {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!(
            "Use of `functools.lru_cache` or `functools.cache` on methods can lead to memory leaks"
        )
    }
}

fn is_cache_func(expr: &Expr, semantic: &SemanticModel) -> bool {
    semantic.resolve_call_path(expr).map_or(false, |call_path| {
        matches!(call_path.as_slice(), ["functools", "lru_cache" | "cache"])
    })
}

/// B019
pub(crate) fn cached_instance_method(checker: &mut Checker, decorator_list: &[Decorator]) {
    if !checker.semantic().scope().kind.is_class() {
        return;
    }
    for decorator in decorator_list {
        // TODO(charlie): This should take into account `classmethod-decorators` and
        // `staticmethod-decorators`.
        if let Expr::Name(ast::ExprName { id, .. }) = &decorator.expression {
            if id == "classmethod" || id == "staticmethod" {
                return;
            }
        }
    }
    for decorator in decorator_list {
        if is_cache_func(
            match &decorator.expression {
                Expr::Call(ast::ExprCall { func, .. }) => func,
                _ => &decorator.expression,
            },
            checker.semantic(),
        ) {
            checker
                .diagnostics
                .push(Diagnostic::new(CachedInstanceMethod, decorator.range()));
        }
    }
}
