use ruff_diagnostics::{AlwaysAutofixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};
use rustpython_parser::ast::{Constant, Expr, ExprCall, ExprConstant, Ranged};

use crate::checkers::ast::Checker;
use crate::registry::AsRule;

/// ## What it does
/// Checks for pathlib `Path` objects that are initialized with the current directory. 
///
/// ## Why is this bad?
/// The `Path()` constructor defaults to the current directory. There is no
/// need to pass the current directory (`"."`) explicitly.
///
/// ## Example
/// ```python
/// from pathlib import Path
///
/// _ = Path(".")
/// ```
///
/// Use instead:
/// ```python
/// from pathlib import Path
///
/// _ = Path()
/// ```
///
/// ## References
/// - [Python documentation: `Path`](https://docs.python.org/3/library/pathlib.html#pathlib.Path)
#[violation]
pub struct PathConstructorCurrentDirectory;

impl AlwaysAutofixableViolation for PathConstructorCurrentDirectory {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Do not pass the current directory explicitly to `Path`")
    }

    fn autofix_title(&self) -> String {
        "Remove the current directory constructor argument".to_string()
    }
}

/// PTH201
pub(crate) fn path_constructor_current_directory(checker: &mut Checker, expr: &Expr, func: &Expr) {
    if !checker
        .semantic()
        .resolve_call_path(func)
        .map_or(false, |call_path| {
            matches!(call_path.as_slice(), ["pathlib", "Path" | "PurePath"])
        })
    {
        return;
    }

    if let Expr::Call(ExprCall { args, keywords, .. }) = expr {
        if !keywords.is_empty() || args.len() != 1 {
            return;
        }
        let arg = args.first().unwrap();
        if let Expr::Constant(ExprConstant {
            value: Constant::Str(value),
            ..
        }) = arg
        {
            if value == "." {
                let mut diagnostic = Diagnostic::new(PathConstructorCurrentDirectory, arg.range());
                if checker.patch(diagnostic.kind.rule()) {
                    diagnostic.set_fix(Fix::automatic(Edit::range_deletion(arg.range())));
                }
                checker.diagnostics.push(diagnostic);
            }
        }
    }
}
