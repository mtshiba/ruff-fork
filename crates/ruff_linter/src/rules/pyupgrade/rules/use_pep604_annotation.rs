use ruff_diagnostics::{
    Applicability, Diagnostic, DiagnosticKind, Edit, Fix, FixAvailability, Violation,
};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::helpers::{pep_604_optional, pep_604_union};
use ruff_python_ast::python_version::PythonVersion;
use ruff_python_ast::{self as ast, Expr};
use ruff_python_semantic::analyze::typing::Pep604Operator;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::codes::Rule;
use crate::fix::edits::pad;
use crate::settings::types::PreviewMode;

/// ## What it does
/// Check for type annotations that can be rewritten based on [PEP 604] syntax.
///
/// ## Why is this bad?
/// [PEP 604] introduced a new syntax for union type annotations based on the
/// `|` operator. This syntax is more concise and readable than the previous
/// `typing.Union` and `typing.Optional` syntaxes.
///
/// This rule is enabled when targeting Python 3.10 or later (see:
/// [`target-version`]). By default, it's _also_ enabled for earlier Python
/// versions if `from __future__ import annotations` is present, as
/// `__future__` annotations are not evaluated at runtime. If your code relies
/// on runtime type annotations (either directly or via a library like
/// Pydantic), you can disable this behavior for Python versions prior to 3.10
/// by setting [`lint.pyupgrade.keep-runtime-typing`] to `true`.
///
/// ## Example
/// ```python
/// from typing import Union
///
/// foo: Union[int, str] = 1
/// ```
///
/// Use instead:
/// ```python
/// foo: int | str = 1
/// ```
///
/// ## Preview
/// In preview mode, this rule only checks for usages of `typing.Union`,
/// while `UP045` checks for `typing.Optional`.
///
/// ## Fix safety
/// This rule's fix is marked as unsafe, as it may lead to runtime errors when
/// alongside libraries that rely on runtime type annotations, like Pydantic,
/// on Python versions prior to Python 3.10. It may also lead to runtime errors
/// in unusual and likely incorrect type annotations where the type does not
/// support the `|` operator.
///
/// ## Options
/// - `target-version`
/// - `lint.pyupgrade.keep-runtime-typing`
///
/// [PEP 604]: https://peps.python.org/pep-0604/
#[derive(ViolationMetadata)]
pub(crate) struct NonPEP604AnnotationUnion;

impl Violation for NonPEP604AnnotationUnion {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Use `X | Y` for type annotations".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Convert to `X | Y`".to_string())
    }
}

/// ## What it does
/// Check for `typing.Optional` annotations that can be rewritten based on [PEP 604] syntax.
///
/// ## Why is this bad?
/// [PEP 604] introduced a new syntax for union type annotations based on the
/// `|` operator. This syntax is more concise and readable than the previous
/// `typing.Optional` syntax.
///
/// This rule is enabled when targeting Python 3.10 or later (see:
/// [`target-version`]). By default, it's _also_ enabled for earlier Python
/// versions if `from __future__ import annotations` is present, as
/// `__future__` annotations are not evaluated at runtime. If your code relies
/// on runtime type annotations (either directly or via a library like
/// Pydantic), you can disable this behavior for Python versions prior to 3.10
/// by setting [`lint.pyupgrade.keep-runtime-typing`] to `true`.
///
/// ## Example
/// ```python
/// from typing import Optional
///
/// foo: Optional[int] = None
/// ```
///
/// Use instead:
/// ```python
/// foo: int | None = None
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as unsafe, as it may lead to runtime errors when
/// alongside libraries that rely on runtime type annotations, like Pydantic,
/// on Python versions prior to Python 3.10. It may also lead to runtime errors
/// in unusual and likely incorrect type annotations where the type does not
/// support the `|` operator.
///
/// ## Options
/// - `target-version`
/// - `lint.pyupgrade.keep-runtime-typing`
///
/// [PEP 604]: https://peps.python.org/pep-0604/
#[derive(ViolationMetadata)]
pub(crate) struct NonPEP604AnnotationOptional;

impl Violation for NonPEP604AnnotationOptional {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Use `X | None` for type annotations".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Convert to `X | None`".to_string())
    }
}

/// UP007, UP045
pub(crate) fn non_pep604_annotation(
    checker: &Checker,
    expr: &Expr,
    slice: &Expr,
    operator: Pep604Operator,
) {
    // Avoid fixing forward references, types not in an annotation, and expressions that would
    // lead to invalid syntax.
    let fixable = checker.semantic().in_type_definition()
        && !checker.semantic().in_complex_string_type_definition()
        && is_allowed_value(slice);

    let applicability = if checker.settings.target_version >= PythonVersion::PY310 {
        Applicability::Safe
    } else {
        Applicability::Unsafe
    };

    match operator {
        Pep604Operator::Optional => {
            let (rule, diagnostic_kind) = match checker.settings.preview {
                PreviewMode::Disabled => (
                    Rule::NonPEP604AnnotationUnion,
                    DiagnosticKind::from(NonPEP604AnnotationUnion),
                ),
                PreviewMode::Enabled => (
                    Rule::NonPEP604AnnotationOptional,
                    DiagnosticKind::from(NonPEP604AnnotationOptional),
                ),
            };

            if !checker.enabled(rule) {
                return;
            }

            let mut diagnostic = Diagnostic::new(diagnostic_kind, expr.range());

            if fixable {
                match slice {
                    Expr::Tuple(_) => {
                        // Invalid type annotation.
                    }
                    _ => {
                        diagnostic.set_fix(Fix::applicable_edit(
                            Edit::range_replacement(
                                pad(
                                    checker.generator().expr(&pep_604_optional(slice)),
                                    expr.range(),
                                    checker.locator(),
                                ),
                                expr.range(),
                            ),
                            applicability,
                        ));
                    }
                }
            }
            checker.report_diagnostic(diagnostic);
        }
        Pep604Operator::Union => {
            if !checker.enabled(Rule::NonPEP604AnnotationUnion) {
                return;
            }

            let mut diagnostic = Diagnostic::new(NonPEP604AnnotationUnion, expr.range());
            if fixable {
                match slice {
                    Expr::Slice(_) => {
                        // Invalid type annotation.
                    }
                    Expr::Tuple(ast::ExprTuple { elts, .. }) => {
                        diagnostic.set_fix(Fix::applicable_edit(
                            Edit::range_replacement(
                                pad(
                                    checker.generator().expr(&pep_604_union(elts)),
                                    expr.range(),
                                    checker.locator(),
                                ),
                                expr.range(),
                            ),
                            applicability,
                        ));
                    }
                    _ => {
                        // Single argument.
                        diagnostic.set_fix(Fix::applicable_edit(
                            Edit::range_replacement(
                                pad(
                                    checker.locator().slice(slice).to_string(),
                                    expr.range(),
                                    checker.locator(),
                                ),
                                expr.range(),
                            ),
                            applicability,
                        ));
                    }
                }
            }
            checker.report_diagnostic(diagnostic);
        }
    }
}

/// Returns `true` if the expression is valid for use in a bitwise union (e.g., `X | Y`). Returns
/// `false` for lambdas, yield expressions, and other expressions that are invalid in such a
/// context.
fn is_allowed_value(expr: &Expr) -> bool {
    // TODO(charlie): If the expression requires parentheses when multi-line, and the annotation
    // itself is not parenthesized, this should return `false`. Consider, for example:
    // ```python
    // x: Union[
    //     "Sequence["
    //         "int"
    //     "]",
    //     float,
    // ]
    // ```
    // Converting this to PEP 604 syntax requires that the multiline string is parenthesized.
    match expr {
        Expr::BoolOp(_)
        | Expr::BinOp(_)
        | Expr::UnaryOp(_)
        | Expr::If(_)
        | Expr::Dict(_)
        | Expr::Set(_)
        | Expr::ListComp(_)
        | Expr::SetComp(_)
        | Expr::DictComp(_)
        | Expr::Generator(_)
        | Expr::Compare(_)
        | Expr::Call(_)
        | Expr::FString(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_)
        | Expr::Attribute(_)
        | Expr::Subscript(_)
        | Expr::Name(_)
        | Expr::List(_) => true,
        Expr::Tuple(tuple) => tuple.iter().all(is_allowed_value),
        // Maybe require parentheses.
        Expr::Named(_) => false,
        // Invalid in binary expressions.
        Expr::Await(_)
        | Expr::Lambda(_)
        | Expr::Yield(_)
        | Expr::YieldFrom(_)
        | Expr::Starred(_)
        | Expr::Slice(_)
        | Expr::IpyEscapeCommand(_) => false,
    }
}
