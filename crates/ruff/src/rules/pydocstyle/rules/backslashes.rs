use memchr::memchr_iter;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;
use crate::docstrings::Docstring;

/// ## What it does
/// Checks for docstrings that include backslashes, but are not defined as
/// raw string literals.
///
/// ## Why is this bad?
/// In Python, backslashes are typically used to escape characters in strings.
/// In raw strings (those prefixed with an `r`), however, backslashes are
/// treated as literal characters.
///
/// [PEP 257](https://peps.python.org/pep-0257/#what-is-a-docstring) recommends
/// the use of raw strings (i.e., `r"""raw triple double quotes"""`) for
/// docstrings that include backslashes. The use of a raw string ensures that
/// any backslashes are treated as literal characters, and not as escape
/// sequences, which avoids confusion.
///
/// ## Example
/// ```python
/// def foobar():
///     """Docstring for foo\bar."""
///
///
/// foobar.__doc__  # "Docstring for foar."
/// ```
///
/// Use instead:
/// ```python
/// def foobar():
///     r"""Docstring for foo\bar."""
///
///
/// foobar.__doc__  # "Docstring for foo\bar."
/// ```
///
/// ## References
/// - [PEP 257 – Docstring Conventions](https://peps.python.org/pep-0257/)
/// - [Python documentation: String and Bytes literals](https://docs.python.org/3/reference/lexical_analysis.html#string-and-bytes-literals)
#[violation]
pub struct EscapeSequenceInDocstring;

impl Violation for EscapeSequenceInDocstring {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!(r#"Use `r"""` if any backslashes in a docstring"#)
    }
}

/// D301
pub(crate) fn backslashes(checker: &mut Checker, docstring: &Docstring) {
    // Docstring is already raw.
    let contents = docstring.contents;
    if contents.starts_with('r') || contents.starts_with("ur") {
        return;
    }

    // Docstring contains at least one backslash.
    let body = docstring.body();
    let bytes = body.as_bytes();
    if memchr_iter(b'\\', bytes).any(|position| {
        let escaped_char = bytes.get(position.saturating_add(1));
        // Allow continuations (backslashes followed by newlines) and Unicode escapes.
        !matches!(escaped_char, Some(b'\r' | b'\n' | b'u' | b'N'))
    }) {
        checker.diagnostics.push(Diagnostic::new(
            EscapeSequenceInDocstring,
            docstring.range(),
        ));
    }
}
