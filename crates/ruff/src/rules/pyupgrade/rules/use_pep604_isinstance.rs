use std::fmt;

use ruff_text_size::TextRange;
use rustpython_parser::ast::{self, Expr, Operator, Ranged};

use ruff_diagnostics::{AlwaysAutofixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;
use crate::registry::AsRule;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum CallKind {
    Isinstance,
    Issubclass,
}

impl fmt::Display for CallKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CallKind::Isinstance => fmt.write_str("isinstance"),
            CallKind::Issubclass => fmt.write_str("issubclass"),
        }
    }
}

impl CallKind {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "isinstance" => Some(CallKind::Isinstance),
            "issubclass" => Some(CallKind::Issubclass),
            _ => None,
        }
    }
}

/// ## What it does
/// Checks for `isinstance` or `issubclass` calls with a tuple of types.
///
/// ## Why is this bad?
/// Since Python 3.10, `isinstance` and `issubclass` can be used with a
/// `|`-separated union of types. This is more concise and easier to read.
///
/// ## Example
/// ```python
/// isinstance(x, (int, float))
/// ```
///
/// Use instead:
/// ```python
/// isinstance(x, int | float)
/// ```
///
/// ## References
/// - [Python documentation: `isinstance`](https://docs.python.org/3/library/functions.html#isinstance)
/// - [Python documentation: `issubclass`](https://docs.python.org/3/library/functions.html#issubclass)
/// - [Python documentation: PEP 604](https://peps.python.org/pep-0604/)
#[violation]
pub struct NonPEP604Isinstance {
    kind: CallKind,
}

impl AlwaysAutofixableViolation for NonPEP604Isinstance {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Use `X | Y` in `{}` call instead of `(X, Y)`", self.kind)
    }

    fn autofix_title(&self) -> String {
        "Convert to `X | Y`".to_string()
    }
}

fn union(elts: &[Expr]) -> Expr {
    if elts.len() == 1 {
        elts[0].clone()
    } else {
        Expr::BinOp(ast::ExprBinOp {
            left: Box::new(union(&elts[..elts.len() - 1])),
            op: Operator::BitOr,
            right: Box::new(elts[elts.len() - 1].clone()),
            range: TextRange::default(),
        })
    }
}

/// UP038
pub(crate) fn use_pep604_isinstance(
    checker: &mut Checker,
    expr: &Expr,
    func: &Expr,
    args: &[Expr],
) {
    if let Expr::Name(ast::ExprName { id, .. }) = func {
        let Some(kind) = CallKind::from_name(id) else {
            return;
        };
        if !checker.semantic().is_builtin(id) {
            return;
        };
        if let Some(types) = args.get(1) {
            if let Expr::Tuple(ast::ExprTuple { elts, .. }) = &types {
                // Ex) `()`
                if elts.is_empty() {
                    return;
                }

                // Ex) `(*args,)`
                if elts.iter().any(Expr::is_starred_expr) {
                    return;
                }

                let mut diagnostic = Diagnostic::new(NonPEP604Isinstance { kind }, expr.range());
                if checker.patch(diagnostic.kind.rule()) {
                    diagnostic.set_fix(Fix::suggested(Edit::range_replacement(
                        checker.generator().expr(&union(elts)),
                        types.range(),
                    )));
                }
                checker.diagnostics.push(diagnostic);
            }
        }
    }
}
