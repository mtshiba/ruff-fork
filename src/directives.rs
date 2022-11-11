//! Extract `# noqa` and `# isort: skip` directives from tokenized source.

use bitflags::bitflags;
use nohash_hasher::{IntMap, IntSet};
use rustpython_ast::Location;
use rustpython_parser::lexer::{LexResult, Tok};

use crate::ast::types::Range;
use crate::checks::LintSource;
use crate::{Settings, SourceCodeLocator};

bitflags! {
    pub struct Flags: u32 {
        const NOQA = 0b00000001;
        const ISORT = 0b00000010;
    }
}

impl Flags {
    pub fn from_settings(settings: &Settings) -> Self {
        if settings
            .enabled
            .iter()
            .any(|check_code| matches!(check_code.lint_source(), LintSource::Imports))
        {
            Flags::NOQA | Flags::ISORT
        } else {
            Flags::NOQA
        }
    }
}

pub struct Directives {
    pub noqa_line_for: Vec<usize>,
    pub isort_exclusions: IntSet<usize>,
}

pub fn extract_directives(
    lxr: &[LexResult],
    locator: &SourceCodeLocator,
    flags: &Flags,
) -> Directives {
    Directives {
        noqa_line_for: if flags.contains(Flags::NOQA) {
            extract_noqa_line_for(lxr)
        } else {
            Default::default()
        },
        isort_exclusions: if flags.contains(Flags::ISORT) {
            extract_isort_exclusions(lxr, locator)
        } else {
            Default::default()
        },
    }
}

/// Extract a mapping from logical line to noqa line.
pub fn extract_noqa_line_for(lxr: &[LexResult]) -> Vec<usize> {
    let mut noqa_line_for: Vec<usize> = vec![];
    for (start, tok, end) in lxr.iter().flatten() {
        if matches!(tok, Tok::EndOfFile) {
            break;
        }
        // For multi-line strings, we expect `noqa` directives on the last line of the
        // string. By definition, we can't have multiple multi-line strings on
        // the same line, so we don't need to verify that we haven't already
        // traversed past the current line.
        if matches!(tok, Tok::String { .. }) && end.row() > start.row() {
            for i in (noqa_line_for.len())..(start.row() - 1) {
                noqa_line_for.push(i + 1);
            }
            noqa_line_for.extend(vec![end.row(); (end.row() + 1) - start.row()]);
        }
    }
    noqa_line_for
}

/// Extract a set of lines over which to disable isort.
pub fn extract_isort_exclusions(lxr: &[LexResult], locator: &SourceCodeLocator) -> IntSet<usize> {
    let mut exclusions: IntSet<usize> = IntSet::default();
    let mut off: Option<&Location> = None;
    for (start, tok, end) in lxr.iter().flatten() {
        // TODO(charlie): Modify RustPython to include the comment text in the token.
        if matches!(tok, Tok::Comment) {
            let comment_text = locator.slice_source_code_range(&Range {
                location: *start,
                end_location: *end,
            });
            if off.is_some() {
                if comment_text == "# isort: on" {
                    if let Some(start) = off {
                        for row in start.row() + 1..=end.row() {
                            exclusions.insert(row);
                        }
                    }
                    off = None;
                }
            } else {
                if comment_text.contains("isort: skip") || comment_text.contains("isort:skip") {
                    exclusions.insert(start.row());
                } else if comment_text == "# isort: off" {
                    off = Some(start);
                }
            }
        } else if matches!(tok, Tok::EndOfFile) {
            if let Some(start) = off {
                for row in start.row() + 1..=end.row() {
                    exclusions.insert(row);
                }
            }
            break;
        }
    }
    exclusions
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rustpython_parser::lexer;
    use rustpython_parser::lexer::LexResult;

    use crate::directives::extract_noqa_line_for;

    #[test]
    fn extraction() -> Result<()> {
        let empty: Vec<usize> = Default::default();

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = 1
y = 2
z = x + 1",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), empty);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "
x = 1
y = 2
z = x + 1",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), empty);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = 1
y = 2
z = x + 1
        ",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), empty);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = 1

y = 2
z = x + 1
        ",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), empty);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = '''abc
def
ghi
'''
y = 2
z = x + 1",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), vec![4, 4, 4, 4]);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = 1
y = '''abc
def
ghi
'''
z = 2",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), vec![1, 5, 5, 5, 5]);

        let lxr: Vec<LexResult> = lexer::make_tokenizer(
            "x = 1
y = '''abc
def
ghi
'''",
        )
        .collect();
        assert_eq!(extract_noqa_line_for(&lxr), vec![1, 5, 5, 5, 5]);

        Ok(())
    }
}
