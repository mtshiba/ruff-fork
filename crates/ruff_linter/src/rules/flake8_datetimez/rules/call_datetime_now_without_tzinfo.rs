use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use ruff_python_ast::{self as ast, Expr};
use ruff_python_semantic::Modules;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::rules::flake8_datetimez::rules::helpers::has_non_none_keyword;

use super::helpers;

/// ## What it does
/// Checks for usage of `datetime.datetime.now()` without a `tz` argument.
///
/// ## Why is this bad?
/// Python datetime objects can be naive or timezone-aware. While an aware
/// object represents a specific moment in time, a naive object does not
/// contain enough information to unambiguously locate itself relative to other
/// datetime objects. Since this can lead to errors, it is recommended to
/// always use timezone-aware objects.
///
/// `datetime.datetime.now()` returns a naive datetime object. Instead, use
/// `datetime.datetime.now(tz=)` to return a timezone-aware object.
///
/// ## Example
/// ```python
/// import datetime
///
/// datetime.datetime.now()
/// ```
///
/// Use instead:
/// ```python
/// import datetime
///
/// datetime.datetime.now(tz=datetime.timezone.utc)
/// ```
///
/// Or, for Python 3.11 and later:
/// ```python
/// import datetime
///
/// datetime.datetime.now(tz=datetime.UTC)
/// ```
///
/// ## Why can't I use `datetime.datetime.now(tz=None)`?
/// ```python
/// datetime.datetime.now(tz=None)  # Returns a naive datetime for the machine's timezone.
/// # So, for a timezone-aware datetime for the machine's timezone, use:
/// datetime.datetime.now(tz=datetime.timezone.utc).astimezone()
/// ```
///
/// ## References
/// - [Python documentation: Aware and Naive Objects](https://docs.python.org/3/library/datetime.html#aware-and-naive-objects)
#[violation]
pub struct CallDatetimeNowWithoutTzinfo;

impl Violation for CallDatetimeNowWithoutTzinfo {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("The use of `datetime.datetime.now()` without `tz` argument is not allowed")
    }
}

pub(crate) fn call_datetime_now_without_tzinfo(checker: &mut Checker, call: &ast::ExprCall) {
    if !checker.semantic().seen_module(Modules::DATETIME) {
        return;
    }

    if !checker
        .semantic()
        .resolve_qualified_name(&call.func)
        .is_some_and(|qualified_name| {
            matches!(qualified_name.segments(), ["datetime", "datetime", "now"])
        })
    {
        return;
    }

    if helpers::parent_expr_is_astimezone(checker) {
        return;
    }

    // no args / no args unqualified
    if call.arguments.args.is_empty() && call.arguments.keywords.is_empty() {
        checker
            .diagnostics
            .push(Diagnostic::new(CallDatetimeNowWithoutTzinfo, call.range()));
        return;
    }

    // none args
    if call
        .arguments
        .args
        .first()
        .is_some_and(Expr::is_none_literal_expr)
    {
        checker
            .diagnostics
            .push(Diagnostic::new(CallDatetimeNowWithoutTzinfo, call.range()));
        return;
    }

    // wrong keywords / none keyword
    if !call.arguments.keywords.is_empty() && !has_non_none_keyword(&call.arguments, "tz") {
        checker
            .diagnostics
            .push(Diagnostic::new(CallDatetimeNowWithoutTzinfo, call.range()));
    }
}
