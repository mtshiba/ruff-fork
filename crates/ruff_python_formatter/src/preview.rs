//! Helpers to test if a specific preview style is enabled or not.
//!
//! The motivation for these functions isn't to avoid code duplication but to ease promoting preview styles
//! to stable. The challenge with directly using [`is_preview`](PyFormatContext::is_preview) is that it is unclear
//! for which specific feature this preview check is for. Having named functions simplifies the promotion:
//! Simply delete the function and let Rust tell you which checks you have to remove.

use crate::PyFormatContext;

/// Returns `true` if the [`hug_parens_with_braces_and_square_brackets`](https://github.com/astral-sh/ruff/issues/8279) preview style is enabled.
pub(crate) const fn is_hug_parens_with_braces_and_square_brackets_enabled(
    context: &PyFormatContext,
) -> bool {
    context.is_preview()
}

/// Returns `true` if the [`f-string formatting`](https://github.com/astral-sh/ruff/issues/7594) preview style is enabled.
/// WARNING: This preview style depends on `is_f_string_implicit_concatenated_string_literal_quotes_enabled`.
/// TODO: Remove `Quoting` when promoting this preview style and convert `FormatStringPart` etc. regular `FormatWithRule` implementations.
/// TODO: Remove `format_f_string` from `normalize_string` when promoting this preview style.
pub(crate) fn is_f_string_formatting_enabled(context: &PyFormatContext) -> bool {
    context.is_preview()
}

/// See [#13539](https://github.com/astral-sh/ruff/pull/13539)
/// Remove `Quoting` when stabilizing this preview style.
pub(crate) fn is_f_string_implicit_concatenated_string_literal_quotes_enabled(
    context: &PyFormatContext,
) -> bool {
    context.is_preview()
}

/// Returns `true` if implicitly concatenated strings should be joined if they all fit on a single line.
/// See [#9457](https://github.com/astral-sh/ruff/issues/9457)
/// WARNING: This preview style depends on `is_empty_parameters_no_unnecessary_parentheses_around_return_value_enabled`
/// because it relies on the new semantic of `IfBreaksParenthesized`.
pub(crate) fn is_join_implicit_concatenated_string_enabled(context: &PyFormatContext) -> bool {
    context.is_preview()
}

/// Returns `true` if the bugfix for single-with items with a trailing comment targeting Python 3.9 or newer is enabled.
///
/// See [#14001](https://github.com/astral-sh/ruff/issues/14001)
pub(crate) fn is_with_single_target_parentheses_enabled(context: &PyFormatContext) -> bool {
    context.is_preview()
}
