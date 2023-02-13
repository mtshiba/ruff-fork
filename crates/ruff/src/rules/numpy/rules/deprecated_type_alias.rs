use ruff_macros::{define_violation, derive_message_formats};
use rustpython_parser::ast::Expr;

use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::fix::Fix;
use crate::registry::Diagnostic;
use crate::violation::AlwaysAutofixableViolation;

define_violation!(
    /// ## What it does
    ///
    /// Checks for deprecated numpy type aliases.
    ///
    /// ## Why is this bad?
    ///
    /// For a long time, np.int has been an alias of the builtin int.
    /// This is repeatedly a cause of confusion for newcomers, and existed mainly for historic reasons.
    /// These aliases have been deprecated in 1.20, and removed in 1.24.
    ///
    /// ## Examples
    ///
    /// ```python
    /// numpy.bool
    /// ```
    ///
    /// Use instead:
    ///
    /// ```python
    /// bool
    /// ```
    ///
    pub struct DeprecatedTypeAlias {
        pub type_name: String,
    }
);
impl AlwaysAutofixableViolation for DeprecatedTypeAlias {
    #[derive_message_formats]
    fn message(&self) -> String {
        let DeprecatedTypeAlias { type_name } = self;
        format!("Numpy type alias `numpy.{type_name}` is deprecated, replace with builtin type")
    }

    fn autofix_title(&self) -> String {
        let DeprecatedTypeAlias { type_name } = self;
        format!("Replace `numpy.{type_name}` with builtin type")
    }
}

/// NPY001
pub fn deprecated_type_alias(checker: &mut Checker, expr: &Expr) {
    if let Some(type_name) = checker.resolve_call_path(expr).and_then(|call_path| {
        if call_path.as_slice() == ["numpy", "bool"]
            || call_path.as_slice() == ["numpy", "int"]
            || call_path.as_slice() == ["numpy", "float"]
            || call_path.as_slice() == ["numpy", "complex"]
            || call_path.as_slice() == ["numpy", "object"]
            || call_path.as_slice() == ["numpy", "str"]
            || call_path.as_slice() == ["numpy", "long"]
            || call_path.as_slice() == ["numpy", "unicode"]
        {
            Some(call_path[1])
        } else {
            None
        }
    }) {
        let mut diagnostic = Diagnostic::new(
            DeprecatedTypeAlias {
                type_name: type_name.to_string(),
            },
            Range::from_located(expr),
        );
        if checker.patch(diagnostic.kind.rule()) {
            diagnostic.amend(Fix::replacement(
                match type_name {
                    "unicode" => "str",
                    "long" => "int",
                    _ => type_name,
                }
                .to_string(),
                expr.location,
                expr.end_location.unwrap(),
            ));
        }
        checker.diagnostics.push(diagnostic);
    }
}
