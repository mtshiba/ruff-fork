use num_traits::ToPrimitive;
use ruff_diagnostics::{AlwaysAutofixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};
use rustpython_parser::ast::{self, Comprehension, Constant, Expr};

use crate::checkers::ast::Checker;
use crate::registry::AsRule;

/// ## What it does
/// Ensures that instead of creating a new list and indexing into it to find the first element of a
/// collection (e.g., `list(...)[0]`), Python iterators are used.
///
/// ## Why is this bad?
/// Creating a new list of great size can involve significant memory/speed concerns. Python's `next(iter(...))`
/// pattern can be used in lieu of creating a new list. This pattern will lazily fetch the first
/// element of the collection, avoiding the memory overhead involved with new list allocation.
/// `next(iter(...))` also is much faster since the list itself doesn't get initialized at once.
///
/// ## Example
/// ```python
/// x = range(1000000000000)
/// return list(x)[0]
/// ```
///
/// Use instead:
/// ```python
/// x = range(1000000000000)
/// return next(iter(x))
/// ```
///
/// ## References
/// - [Iterators and Iterables in Python: Run Efficient
/// Iterations](https://realpython.com/python-iterators-iterables/#when-to-use-an-iterator-in-python)
#[violation]
pub(crate) struct UnnecessaryIterableAllocationForFirstElement {
    arg: String,
    contains_slice: bool,
}

impl UnnecessaryIterableAllocationForFirstElement {
    pub(crate) fn new(arg: String, contains_slice: bool) -> Self {
        Self {
            arg,
            contains_slice,
        }
    }
}

impl AlwaysAutofixableViolation for UnnecessaryIterableAllocationForFirstElement {
    #[derive_message_formats]
    fn message(&self) -> String {
        if self.contains_slice {
            format!(
                "Prefer `[next(iter({}))]` over `list({})[:1]` or equivalent list comprehension/slice",
                self.arg, self.arg
            )
        } else {
            format!(
                "Prefer `next(iter({}))` over `list({})[0]` or equivalent list comprehension/slice",
                self.arg, self.arg
            )
        }
    }

    fn autofix_title(&self) -> String {
        if self.contains_slice {
            format!("Replace with `[next(iter({}))]", self.arg)
        } else {
            format!("Replace with `next(iter({}))`", self.arg)
        }
    }
}

/// Contains information about a [`Expr::Subscript`] to determine if and how it accesses the first element
/// of an iterable.
struct ClassifiedSubscript {
    /// If the subscript accesses the first element of the iterable
    indexes_first_element: bool,
    /// If the subscript is a slice (e.g., `[:1]`)
    is_slice: bool,
}

impl ClassifiedSubscript {
    fn new(indexes_first_element: bool, is_slice: bool) -> Self {
        Self {
            indexes_first_element,
            is_slice,
        }
    }
}

/// RUF015
pub(crate) fn unnecessary_iterable_allocation_for_first_element(
    checker: &mut Checker,
    subscript: &Expr,
) {
    let Expr::Subscript(ast::ExprSubscript { value, slice, range, .. }) = subscript else {
        return;
    };

    let ClassifiedSubscript {
        indexes_first_element,
        is_slice,
    } = classify_subscript(slice);
    if !indexes_first_element {
        return;
    }

    let Some(iter_name) = get_iterable_name(checker, value) else {
        return;
    };

    let mut diagnostic = Diagnostic::new(
        UnnecessaryIterableAllocationForFirstElement::new(iter_name.to_string(), is_slice),
        *range,
    );

    if checker.patch(diagnostic.kind.rule()) {
        let replacement = if is_slice {
            format!("[next(iter({iter_name}))]")
        } else {
            format!("next(iter({iter_name}))")
        };

        diagnostic.set_fix(Fix::suggested(Edit::range_replacement(replacement, *range)));
    }

    checker.diagnostics.push(diagnostic);
}

/// Check that the slice [`Expr`] is functionally equivalent to slicing into the first element. The
/// first `bool` checks that the element is in fact first, the second checks if it's a slice or an
/// index.
fn classify_subscript(expr: &Expr) -> ClassifiedSubscript {
    match expr {
        Expr::Constant(ast::ExprConstant { .. }) => {
            let effective_index = get_effective_index(expr);
            ClassifiedSubscript::new(matches!(effective_index, None | Some(0)), false)
        }
        Expr::Slice(ast::ExprSlice {
            step: step_value,
            lower: lower_index,
            upper: upper_index,
            ..
        }) => {
            let lower = lower_index.as_ref().and_then(|l| get_effective_index(l));
            let upper = upper_index.as_ref().and_then(|u| get_effective_index(u));
            let step = step_value.as_ref().and_then(|s| get_effective_index(s));

            if matches!(lower, None | Some(0)) {
                if upper.unwrap_or(i64::MAX) > step.unwrap_or(1i64) {
                    return ClassifiedSubscript::new(false, true);
                }

                return ClassifiedSubscript::new(true, true);
            }

            ClassifiedSubscript::new(false, true)
        }
        _ => ClassifiedSubscript::new(false, false),
    }
}

/// Fetch the name of the iterable from an expression if the expression returns an unmodified list
/// which can be sliced into.
fn get_iterable_name<'a>(checker: &mut Checker, expr: &'a Expr) -> Option<&'a str> {
    match expr {
        Expr::Call(ast::ExprCall { func, args, .. }) => {
            let Some(id) = get_name_id(func.as_ref()) else {
                return None;
            };
            if !((id == "list" && checker.semantic().is_builtin("list"))
                || (id == "tuple" && checker.semantic().is_builtin("tuple")))
            {
                return None;
            }

            match args.first() {
                Some(Expr::Name(ast::ExprName { id: arg_name, .. })) => Some(arg_name.as_str()),
                Some(Expr::GeneratorExp(ast::ExprGeneratorExp {
                    elt, generators, ..
                })) => get_generator_iterable(elt, generators),
                _ => None,
            }
        }
        Expr::ListComp(ast::ExprListComp {
            elt, generators, ..
        }) => get_generator_iterable(elt, generators),
        _ => None,
    }
}

fn get_generator_iterable<'a>(
    elt: &'a Expr,
    generators: &'a Vec<Comprehension>,
) -> Option<&'a str> {
    // If the `elt` field is anything other than a [`Expr::Name`], we can't be sure that it
    // doesn't modify the elements of the underlying iterator - for example, `[i + 1 for i in x][0]`.
    if !matches!(elt, Expr::Name(ast::ExprName { .. })) {
        return None;
    }

    // If there's more than 1 generator, we can't safely say that it fits the diagnostic conditions -
    // for example, `[(i, j) for i in x for j in y][0]`.
    if generators.len() != 1 {
        return None;
    }

    let generator = &generators[0];
    // Ignore if there's an `if` statement in the comprehension, since it filters the list.
    if !generator.ifs.is_empty() {
        return None;
    }

    let Some(arg_name) = get_name_id(&generator.iter) else {
                return None;
            };

    Some(arg_name)
}

fn get_name_id(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Name(ast::ExprName { id, .. }) => Some(id),
        _ => None,
    }
}

fn get_effective_index(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Constant(ast::ExprConstant {
            value: Constant::Int(value),
            ..
        }) => value.to_i64(),
        _ => None,
    }
}
