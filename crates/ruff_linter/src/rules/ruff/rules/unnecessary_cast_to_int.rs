use crate::checkers::ast::Checker;
use ruff_diagnostics::{AlwaysFixableViolation, Applicability, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{Arguments, Expr, ExprCall, ExprNumberLiteral, Number};
use ruff_python_semantic::SemanticModel;
use ruff_text_size::TextRange;

/// ## What it does
/// Checks for `int` conversions of values that are already integers.
///
/// ## Why is this bad?
/// Such a conversion is unnecessary.
///
/// ## Known problems
/// This rule is prone to false positives due to type inference limitations.
///
/// It assumes that `round`, `math.ceil`, `math.floor`, `math.trunc`
/// always return `int`, which might not be the case for objects
/// with the corresponding dunder methods overridden.
/// In such cases, the fix is marked as unsafe.
///
/// ## Example
///
/// ```python
/// int(len([]))
/// int(round(foo, 0))
/// ```
///
/// Use instead:
///
/// ```python
/// len([])
/// round(foo)
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct UnnecessaryCastToInt;

impl AlwaysFixableViolation for UnnecessaryCastToInt {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Value being casted is already an integer".to_string()
    }

    fn fix_title(&self) -> String {
        "Remove `int()` wrapper call".to_string()
    }
}

/// RUF046
pub(crate) fn unnecessary_cast_to_int(checker: &mut Checker, call: &ExprCall) {
    let semantic = checker.semantic();

    let Some(Expr::Call(inner_call)) = single_argument_to_int_call(semantic, call) else {
        return;
    };

    let (func, arguments) = (&inner_call.func, &inner_call.arguments);
    let (outer_range, inner_range) = (call.range, inner_call.range);

    let Some(qualified_name) = checker.semantic().resolve_qualified_name(func) else {
        return;
    };

    let (edit, applicability) = match qualified_name.segments() {
        // Always returns a strict instance of `int`
        ["" | "builtins", "len" | "id" | "hash" | "ord" | "int"]
        | ["math", "comb" | "factorial" | "gcd" | "lcm" | "isqrt" | "perm"] => (
            handle_other(checker, outer_range, inner_range),
            Applicability::Safe,
        ),

        // Depends on `ndigits` and `number.__round__`
        ["" | "builtins", "round"] => match handle_round(checker, outer_range, arguments) {
            None => return,
            Some(edit) => (edit, Applicability::Unsafe),
        },

        // Depends on `__ceil__`/`__floor__`/`__trunc__`
        ["math", "ceil" | "floor" | "trunc"] => (
            handle_other(checker, outer_range, inner_range),
            Applicability::Unsafe,
        ),

        _ => return,
    };

    let diagnostic = Diagnostic::new(UnnecessaryCastToInt {}, call.range);
    let fix = Fix::applicable_edit(edit, applicability);

    checker.diagnostics.push(diagnostic.with_fix(fix));
}

fn single_argument_to_int_call<'a>(
    semantic: &SemanticModel,
    call: &'a ExprCall,
) -> Option<&'a Expr> {
    let ExprCall {
        func, arguments, ..
    } = call;

    if !semantic.match_builtin_expr(func, "int") {
        return None;
    }

    if !arguments.keywords.is_empty() {
        return None;
    }

    let [argument] = &*arguments.args else {
        return None;
    };

    Some(argument)
}

fn handle_round(checker: &Checker, outer_range: TextRange, arguments: &Arguments) -> Option<Edit> {
    if arguments.len() > 2 {
        return None;
    }

    let number = arguments.find_argument("number", 0)?;
    let ndigits = arguments.find_argument("ndigits", 1);

    let number_expr = checker.locator().slice(number);
    let new_content = match ndigits {
        Some(Expr::NumberLiteral(ExprNumberLiteral { value, .. })) if is_literal_zero(value) => {
            format!("round({number_expr})")
        }
        Some(Expr::NoneLiteral(_)) | None => format!("round({number_expr})"),
        _ => return None,
    };

    Some(Edit::range_replacement(new_content, outer_range))
}

fn is_literal_zero(value: &Number) -> bool {
    let Number::Int(int) = value else {
        return false;
    };

    matches!(int.as_u8(), Some(0))
}

fn handle_other(checker: &Checker, outer_range: TextRange, inner_range: TextRange) -> Edit {
    let inner_expr = checker.locator().slice(inner_range);

    Edit::range_replacement(inner_expr.to_string(), outer_range)
}
