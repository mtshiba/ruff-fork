use rustpython_parser::ast::Expr;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use rustpython_parser::ast::Ranged;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for uses of `collections.namedtuple` in stub files.
///
/// ## Why is this bad?
/// `typing.NamedTuple` is the "typed version" of `collections.namedtuple`.
///
/// The class generated by subclassing `typing.NamedTuple` is equivalent to
/// `collections.namedtuple`, with the exception that `typing.NamedTuple`
/// includes an `__annotations__` attribute, which allows type checkers to
/// infer the types of the fields.
///
/// ## Example
/// ```python
/// from collections import namedtuple
///
///
/// person = namedtuple("Person", ["name", "age"])
/// ```
///
/// Use instead:
/// ```python
/// from typing import NamedTuple
///
///
/// class Person(NamedTuple):
///     name: str
///     age: int
/// ```
#[violation]
pub struct CollectionsNamedTuple;

impl Violation for CollectionsNamedTuple {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Use `typing.NamedTuple` instead of `collections.namedtuple`")
    }

    fn autofix_title(&self) -> Option<String> {
        Some(format!("Replace with `typing.NamedTuple`"))
    }
}

/// PYI024
pub(crate) fn collections_named_tuple(checker: &mut Checker, expr: &Expr) {
    if checker
        .semantic()
        .resolve_call_path(expr)
        .map_or(false, |call_path| {
            matches!(call_path.as_slice(), ["collections", "namedtuple"])
        })
    {
        checker
            .diagnostics
            .push(Diagnostic::new(CollectionsNamedTuple, expr.range()));
    }
}
