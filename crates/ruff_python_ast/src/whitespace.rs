use rustpython_parser::ast::{Located, Location};
use std::iter::FusedIterator;

use crate::source_code::Locator;
use crate::types::Range;

/// Extract the leading indentation from a line.
pub fn indentation<'a, T>(locator: &'a Locator, located: &'a Located<T>) -> Option<&'a str> {
    let range = Range::from(located);
    let indentation = locator.slice(Range::new(
        Location::new(range.location.row(), 0),
        Location::new(range.location.row(), range.location.column()),
    ));
    if indentation.chars().all(char::is_whitespace) {
        Some(indentation)
    } else {
        None
    }
}

/// Extract the leading words from a line of text.
pub fn leading_words(line: &str) -> &str {
    let line = line.trim();
    line.find(|char: char| !char.is_alphanumeric() && !char.is_whitespace())
        .map_or(line, |index| &line[..index])
}

/// Extract the leading whitespace from a line of text.
pub fn leading_space(line: &str) -> &str {
    line.find(|char: char| !char.is_whitespace())
        .map_or(line, |index| &line[..index])
}

/// Replace any non-whitespace characters from an indentation string.
pub fn clean(indentation: &str) -> String {
    indentation
        .chars()
        .map(|char| if char.is_whitespace() { char } else { ' ' })
        .collect()
}

/// Like [`UniversalNewlineIterator`], but includes a trailing newline as an empty line.
pub struct NewlineWithTrailingNewline<'a> {
    trailing: Option<&'a str>,
    underlying: UniversalNewlineIterator<'a>,
}

impl<'a> NewlineWithTrailingNewline<'a> {
    pub fn from(input: &'a str) -> NewlineWithTrailingNewline<'a> {
        NewlineWithTrailingNewline {
            underlying: UniversalNewlineIterator::from(input),
            trailing: if input.ends_with(['\r', '\n']) {
                Some("")
            } else {
                None
            },
        }
    }
}

impl<'a> Iterator for NewlineWithTrailingNewline<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.underlying.next().or_else(|| self.trailing.take())
    }
}

/// Like [`str#lines`], but accommodates LF, CRLF, and CR line endings,
/// the latter of which are not supported by [`str#lines`].
///
/// ## Examples
///
/// ```rust
/// use ruff_python_ast::whitespace::UniversalNewlineIterator;
///
/// let mut lines = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop");
///
/// assert_eq!(lines.next_back(), Some("bop"));
/// assert_eq!(lines.next(), Some("foo"));
/// assert_eq!(lines.next_back(), Some("baz"));
/// assert_eq!(lines.next(), Some("bar"));
/// assert_eq!(lines.next_back(), Some(""));
/// assert_eq!(lines.next(), None);
/// ```
pub struct UniversalNewlineIterator<'a> {
    text: &'a str,
}

impl<'a> UniversalNewlineIterator<'a> {
    pub fn from(text: &'a str) -> UniversalNewlineIterator<'a> {
        UniversalNewlineIterator { text }
    }
}

impl<'a> Iterator for UniversalNewlineIterator<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.text.is_empty() {
            return None;
        }

        let line = match self.text.find(['\n', '\r']) {
            // Non-last line
            Some(line_end) => {
                let (line, remainder) = self.text.split_at(line_end);

                self.text = match remainder.as_bytes()[0] {
                    // Explicit branch for `\n` as this is the most likely path
                    b'\n' => &remainder[1..],
                    // '\r\n'
                    b'\r' if remainder.as_bytes().get(1) == Some(&b'\n') => &remainder[2..],
                    // '\r'
                    _ => &remainder[1..],
                };

                line
            }
            // Last line
            None => std::mem::take(&mut self.text),
        };

        Some(line)
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl DoubleEndedIterator for UniversalNewlineIterator<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }

        let len = self.text.len();

        // Trim any trailing newlines.
        self.text = match self.text.as_bytes()[len - 1] {
            b'\n' if len > 1 && self.text.as_bytes()[len - 2] == b'\r' => &self.text[..len - 2],
            b'\n' | b'\r' => &self.text[..len - 1],
            _ => self.text,
        };

        // Find the end of the previous line. The previous line is the text up to, but not including
        // the newline character.
        let line = match self.text.rfind(['\n', '\r']) {
            // '\n' or '\r' or '\r\n'
            Some(line_end) => {
                let (remainder, line) = self.text.split_at(line_end + 1);
                self.text = remainder;

                line
            }
            // Last line
            None => std::mem::take(&mut self.text),
        };

        Some(line)
    }
}

impl FusedIterator for UniversalNewlineIterator<'_> {}

#[cfg(test)]
mod tests {
    use super::UniversalNewlineIterator;

    #[test]
    fn universal_newlines_empty_str() {
        let lines: Vec<_> = UniversalNewlineIterator::from("").collect();
        assert_eq!(lines, Vec::<&str>::default());

        let lines: Vec<_> = UniversalNewlineIterator::from("").rev().collect();
        assert_eq!(lines, Vec::<&str>::default());
    }

    #[test]
    fn universal_newlines_forward() {
        let lines: Vec<_> = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop").collect();
        assert_eq!(lines, vec!["foo", "bar", "", "baz", "bop"]);

        let lines: Vec<_> = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop\n").collect();
        assert_eq!(lines, vec!["foo", "bar", "", "baz", "bop"]);

        let lines: Vec<_> = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop\n\n").collect();
        assert_eq!(lines, vec!["foo", "bar", "", "baz", "bop", ""]);
    }

    #[test]
    fn universal_newlines_backwards() {
        let lines: Vec<_> = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop")
            .rev()
            .collect();
        assert_eq!(lines, vec!["bop", "baz", "", "bar", "foo"]);

        let lines: Vec<_> = UniversalNewlineIterator::from("foo\nbar\n\nbaz\rbop\n")
            .rev()
            .collect();

        assert_eq!(lines, vec!["bop", "baz", "", "bar", "foo"]);
    }

    #[test]
    fn universal_newlines_mixed() {
        let mut lines = UniversalNewlineIterator::from("foo\nbar\n\r\nbaz\rbop");

        assert_eq!(lines.next_back(), Some("bop"));
        assert_eq!(lines.next(), Some("foo"));
        assert_eq!(lines.next_back(), Some("baz"));
        assert_eq!(lines.next(), Some("bar"));
        assert_eq!(lines.next_back(), Some(""));
        assert_eq!(lines.next(), None);
    }
}
