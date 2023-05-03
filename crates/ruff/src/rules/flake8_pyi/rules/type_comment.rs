use once_cell::sync::Lazy;
use regex::Regex;
use rustpython_parser::lexer::LexResult;
use rustpython_parser::Tok;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

/// ## What it does
/// Checks for the use of type comments (e.g., `x = 1  # type: int`).
///
/// ## Why is this bad?
/// Stub (`.pyi`) files should use type annotations directly, rather
/// than type comments, even if they're intended to support Python 2, since
/// stub files are not executed at runtime. The one exception is `# type: ignore`.
///
/// It should not be used in `.py` files either in Python 3.6+ versions.
///
/// ## Example
/// ```python
/// x = 1  # type: int
/// ```
///
/// Use instead:
/// ```python
/// x: int = 1
/// ```
#[violation]
pub struct TypeComment;

impl Violation for TypeComment {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Don't use type comments")
    }
}

/// PYI033
pub(crate) fn type_comment(tokens: &[LexResult]) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    for token in tokens.iter().flatten() {
        if let (Tok::Comment(comment), range) = token {
            if TYPE_COMMENT_REGEX.is_match(comment) && !TYPE_IGNORE_REGEX.is_match(comment) {
                diagnostics.push(Diagnostic::new(TypeComment, *range));
            }
        }
    }

    diagnostics
}

static TYPE_COMMENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#\s*type:\s*([^#]+)(\s*#.*?)?$").unwrap());

static TYPE_IGNORE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#\s*type:\s*ignore([^#]+)?(\s*#.*?)?$").unwrap());
