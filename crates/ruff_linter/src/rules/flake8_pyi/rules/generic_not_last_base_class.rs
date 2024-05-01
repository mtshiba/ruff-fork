use ruff_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::{self as ast, Arguments, Expr, StmtClassDef};
use ruff_python_semantic::SemanticModel;
use ruff_source_file::Locator;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for classes inheriting from `typing.Generic[]`, but `Generic[]` is
/// not the last base class in the bases list.
///
/// ## Why is this bad?
/// `Generic[]` not being the final class in the bases tuple can cause
/// unexpected behaviour at runtime (See [this CPython issue][1] for example).
/// In a stub file, however, this rule is enforced purely for stylistic
/// consistency.
///
/// For example:
/// ```python
/// class LinkedList(Generic[T], Sized):
///     def push(self, item: T) -> None:
///         self._items.append(item)
///
/// class MyMapping(
///     Generic[K, V],
///     Iterable[Tuple[K, V]],
///     Container[Tuple[K, V]],
/// ):
///     ...
/// ```
///
/// ```python
/// class LinkedList(Sized, Generic[T]):
///     def push(self, item: T) -> None:
///         self._items.append(item)
///
/// class MyMapping(
///     Iterable[Tuple[K, V]],
///     Container[Tuple[K, V]],
///     Generic[K, V],
/// ):
///     ...
/// ```
/// ## References
/// - [`typing.Generic` documentation](https://docs.python.org/3/library/typing.html#typing.Generic)
///
/// [1]: https://github.com/python/cpython/issues/106102
#[violation]
pub struct GenericNotLastBaseClass;

impl Violation for GenericNotLastBaseClass {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("`Generic[]` should always be the last base class")
    }

    fn fix_title(&self) -> Option<String> {
        Some("Move `Generic[]` to be the last base class".to_string())
    }
}

/// PYI059
pub(crate) fn generic_not_last_base_class(
    checker: &mut Checker,
    class_def: &StmtClassDef,
    bases: Option<&Arguments>,
) {
    let Some(bases) = bases else {
        return;
    };

    let semantic = checker.semantic();

    for (base_index, base) in bases.args.iter().enumerate() {
        if base_index == bases.args.len() - 1 {
            // Don't raise issue if it is the last base.
            return;
        }

        if is_generic(base, semantic) {
            let mut diagnostic = Diagnostic::new(GenericNotLastBaseClass, class_def.identifier());
            diagnostic.set_fix(generate_fix(bases, base_index, checker.locator()));
            checker.diagnostics.push(diagnostic);
            break;
        }
    }
}

/// Return `true` if the given expression resolves to `typing.Generic[...]`.
fn is_generic(expr: &Expr, semantic: &SemanticModel) -> bool {
    if !semantic.seen_typing() {
        return false;
    }

    let Expr::Subscript(ast::ExprSubscript { value, .. }) = expr else {
        return false;
    };

    let qualified_name = semantic.resolve_qualified_name(value);
    qualified_name.as_ref().is_some_and(|qualified_name| {
        semantic.match_typing_qualified_name(qualified_name, "Generic")
    })
}

// let call_start = Edit::deletion(call.start(), argument.start());

// // Delete from the start of the call to the start of the argument.

// // Delete from the end of the argument to the end of the call.
// let call_end = Edit::deletion(argument.end(), call.end());

// // If this is a tuple, we also need to convert the inner argument to a list.
// if argument.is_tuple_expr() {
//     // Replace `(` with `[`.
//     let argument_start = Edit::replacement(
//         "[".to_string(),
//         argument.start(),
//         argument.start() + TextSize::from(1),
//     );

//     // Replace `)` with `]`.
//     let argument_end = Edit::replacement(
//         "]".to_string(),
//         argument.end() - TextSize::from(1),
//         argument.end(),
//     );

//     Fix::unsafe_edits(call_start, [argument_start, argument_end, call_end])
// } else {
//     Fix::unsafe_edits(call_start, [call_end])
// }
fn generate_fix(bases: &Arguments, generic_base_index: usize, locator: &Locator) -> Fix {
    let last_base = bases.args.last().expect("Last base should always exist");
    let generic_base = bases
        .args
        .get(generic_base_index)
        .expect("Generic base should always exist");
    let next_base = bases
        .args
        .get(generic_base_index + 1)
        .expect("Generic base should never be the last base during auto-fix");

    let deletion = Edit::deletion(generic_base.start(), next_base.start());
    let insertion = Edit::insertion(
        format!(", {}", locator.slice(generic_base.range())),
        last_base.end(),
    );
    return Fix::safe_edits(insertion, [deletion]);
}
