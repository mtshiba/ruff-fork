use ruff_python_ast::{self as ast, Expr, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::name::{QualifiedName, UnqualifiedName};
use ruff_python_semantic::analyze::typing::is_immutable_func;
use ruff_python_semantic::SemanticModel;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::rules::ruff::rules::helpers::{
    is_class_var_annotation, is_dataclass, is_dataclass_field, is_descriptor_class,
};
use crate::rules::ruff::rules::is_attrs_dataclass;

/// ## What it does
/// Checks for function calls in dataclass attribute defaults.
///
/// ## Why is this bad?
/// Function calls are only performed once, at definition time. The returned
/// value is then reused by all instances of the dataclass. This can lead to
/// unexpected behavior when the function call returns a mutable object, as
/// changes to the object will be shared across all instances.
///
/// If a field needs to be initialized with a mutable object, use the
/// `field(default_factory=...)` pattern.
///
/// ## Examples
/// ```python
/// from dataclasses import dataclass
///
///
/// def simple_list() -> list[int]:
///     return [1, 2, 3, 4]
///
///
/// @dataclass
/// class A:
///     mutable_default: list[int] = simple_list()
/// ```
///
/// Use instead:
/// ```python
/// from dataclasses import dataclass, field
///
///
/// def creating_list() -> list[int]:
///     return [1, 2, 3, 4]
///
///
/// @dataclass
/// class A:
///     mutable_default: list[int] = field(default_factory=creating_list)
/// ```
///
/// ## Options
/// - `lint.flake8-bugbear.extend-immutable-calls`
#[violation]
pub struct FunctionCallInDataclassDefaultArgument {
    name: Option<String>,
}

impl Violation for FunctionCallInDataclassDefaultArgument {
    #[derive_message_formats]
    fn message(&self) -> String {
        if let Some(name) = &self.name {
            format!("Do not perform function call `{name}` in dataclass defaults")
        } else {
            "Do not perform function call in dataclass defaults".to_string()
        }
    }
}

/// RUF009
pub(crate) fn function_call_in_dataclass_default(
    checker: &mut Checker,
    class_def: &ast::StmtClassDef,
) {
    let semantic = checker.semantic();

    if !is_dataclass(class_def, semantic) && !is_attrs_dataclass(class_def, semantic) {
        return;
    }

    let extend_immutable_calls: Vec<QualifiedName> = checker
        .settings
        .flake8_bugbear
        .extend_immutable_calls
        .iter()
        .map(|target| QualifiedName::from_dotted_name(target))
        .collect();

    for statement in &class_def.body {
        let Stmt::AnnAssign(ast::StmtAnnAssign {
            annotation,
            value: Some(expr),
            ..
        }) = statement
        else {
            continue;
        };
        let Expr::Call(ast::ExprCall { func, .. }) = expr.as_ref() else {
            continue;
        };

        if !is_class_var_annotation(annotation, checker.semantic())
            && !is_immutable_func(func, checker.semantic(), &extend_immutable_calls)
            && !is_dataclass_field(func, checker.semantic())
            && !is_attrs_field(func, checker.semantic())
            && !is_descriptor_class(func, checker.semantic())
        {
            let kind = FunctionCallInDataclassDefaultArgument {
                name: UnqualifiedName::from_expr(func).map(|name| name.to_string()),
            };
            let diagnostic = Diagnostic::new(kind, expr.range());

            checker.diagnostics.push(diagnostic);
        }
    }
}

/// Whether the function call is either `attr.ib()` or `attr.field()`.
fn is_attrs_field(func: &Expr, semantic: &SemanticModel) -> bool {
    let Some(qualified_name) = semantic.resolve_qualified_name(func) else {
        return false;
    };

    matches!(
        qualified_name.segments(),
        ["attrs", "field" | "Factory"] | ["attr", "ib"]
    )
}
