use std::hash::BuildHasherDefault;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::{Arguments, Expr, ExprCall, ExprDict, ExprStringLiteral};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for repeated keyword arguments passed to a function call
///
/// ## Why is this bad?
/// Python does not allow for multiple values to be assigned to the same
/// keyword argument in a single function call.
///
/// ## Example
/// ```python
/// func(1, 2, c=3, **{"c": 4})
/// ```
///
/// Use instead:
/// ```python
/// func(1, 2, **{"c": 4})
/// ```
///
/// ## References
/// - [Python documentation: Argument](https://docs.python.org/3/glossary.html#term-argument)
#[violation]
pub struct RepeatedKeywords {
    duplicate_keyword: String,
}

impl Violation for RepeatedKeywords {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self { duplicate_keyword } = self;
        format!("Repeated keyword argument: `{duplicate_keyword}`")
    }
}

pub(crate) fn repeated_keywords(checker: &mut Checker, call: &ExprCall) {
    let ExprCall {
        arguments: Arguments { keywords, .. },
        ..
    } = call;

    let mut seen =
        FxHashSet::with_capacity_and_hasher(keywords.len(), BuildHasherDefault::default());

    for keyword in keywords {
        if let Some(id) = &keyword.arg {
            if !seen.insert(id.as_str()) {
                checker.diagnostics.push(Diagnostic::new(
                    RepeatedKeywords {
                        duplicate_keyword: id.to_string(),
                    },
                    keyword.range(),
                ));
            }
        // We only want to check dict keys if there is NO arg associated with them
        } else if let Expr::Dict(ExprDict { keys, .. }) = &keyword.value {
            for key in keys.iter().flatten() {
                if let Expr::StringLiteral(ExprStringLiteral { value, .. }) = key {
                    if !seen.insert(value) {
                        checker.diagnostics.push(Diagnostic::new(
                            RepeatedKeywords {
                                duplicate_keyword: value.to_string(),
                            },
                            key.range(),
                        ));
                    }
                }
            }
        }
    }
}
