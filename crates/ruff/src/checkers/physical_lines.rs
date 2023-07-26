//! Lint rules based on checking physical lines.
use ruff_text_size::TextSize;

use ruff_diagnostics::Diagnostic;
use ruff_python::codegen::Stylist;
use ruff_python::index::Indexer;
use ruff_python_trivia::UniversalNewlines;
use ruff_source_file::Locator;

use crate::registry::Rule;
use crate::rules::flake8_copyright::rules::missing_copyright_notice;
use crate::rules::pycodestyle::rules::{
    doc_line_too_long, line_too_long, mixed_spaces_and_tabs, no_newline_at_end_of_file,
    tab_indentation, trailing_whitespace,
};
use crate::rules::pylint;
use crate::rules::pyupgrade::rules::unnecessary_coding_comment;
use crate::settings::Settings;

pub(crate) fn check_physical_lines(
    locator: &Locator,
    stylist: &Stylist,
    indexer: &Indexer,
    doc_lines: &[TextSize],
    settings: &Settings,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];

    let enforce_doc_line_too_long = settings.rules.enabled(Rule::DocLineTooLong);
    let enforce_line_too_long = settings.rules.enabled(Rule::LineTooLong);
    let enforce_no_newline_at_end_of_file = settings.rules.enabled(Rule::MissingNewlineAtEndOfFile);
    let enforce_unnecessary_coding_comment = settings.rules.enabled(Rule::UTF8EncodingDeclaration);
    let enforce_mixed_spaces_and_tabs = settings.rules.enabled(Rule::MixedSpacesAndTabs);
    let enforce_bidirectional_unicode = settings.rules.enabled(Rule::BidirectionalUnicode);
    let enforce_trailing_whitespace = settings.rules.enabled(Rule::TrailingWhitespace);
    let enforce_blank_line_contains_whitespace =
        settings.rules.enabled(Rule::BlankLineWithWhitespace);
    let enforce_tab_indentation = settings.rules.enabled(Rule::TabIndentation);
    let enforce_copyright_notice = settings.rules.enabled(Rule::MissingCopyrightNotice);

    let fix_unnecessary_coding_comment = settings.rules.should_fix(Rule::UTF8EncodingDeclaration);

    let mut commented_lines_iter = indexer.comment_ranges().iter().peekable();
    let mut doc_lines_iter = doc_lines.iter().peekable();

    for (index, line) in locator.contents().universal_newlines().enumerate() {
        while commented_lines_iter
            .next_if(|comment_range| line.range().contains_range(**comment_range))
            .is_some()
        {
            if enforce_unnecessary_coding_comment {
                if index < 2 {
                    if let Some(diagnostic) =
                        unnecessary_coding_comment(&line, fix_unnecessary_coding_comment)
                    {
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        while doc_lines_iter
            .next_if(|doc_line_start| line.range().contains_inclusive(**doc_line_start))
            .is_some()
        {
            if enforce_doc_line_too_long {
                if let Some(diagnostic) = doc_line_too_long(&line, settings) {
                    diagnostics.push(diagnostic);
                }
            }
        }

        if enforce_mixed_spaces_and_tabs {
            if let Some(diagnostic) = mixed_spaces_and_tabs(&line) {
                diagnostics.push(diagnostic);
            }
        }

        if enforce_line_too_long {
            if let Some(diagnostic) = line_too_long(&line, settings) {
                diagnostics.push(diagnostic);
            }
        }

        if enforce_bidirectional_unicode {
            diagnostics.extend(pylint::rules::bidirectional_unicode(&line));
        }

        if enforce_trailing_whitespace || enforce_blank_line_contains_whitespace {
            if let Some(diagnostic) = trailing_whitespace(&line, locator, indexer, settings) {
                diagnostics.push(diagnostic);
            }
        }

        if enforce_tab_indentation {
            if let Some(diagnostic) = tab_indentation(&line, indexer) {
                diagnostics.push(diagnostic);
            }
        }
    }

    if enforce_no_newline_at_end_of_file {
        if let Some(diagnostic) = no_newline_at_end_of_file(
            locator,
            stylist,
            settings.rules.should_fix(Rule::MissingNewlineAtEndOfFile),
        ) {
            diagnostics.push(diagnostic);
        }
    }

    if enforce_copyright_notice {
        if let Some(diagnostic) = missing_copyright_notice(locator, settings) {
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use rustpython_parser::lexer::lex;
    use rustpython_parser::Mode;

    use ruff_python::codegen::Stylist;
    use ruff_python::index::Indexer;
    use ruff_source_file::Locator;

    use crate::line_width::LineLength;
    use crate::registry::Rule;
    use crate::settings::Settings;

    use super::check_physical_lines;

    #[test]
    fn e501_non_ascii_char() {
        let line = "'\u{4e9c}' * 2"; // 7 in UTF-32, 9 in UTF-8.
        let locator = Locator::new(line);
        let tokens: Vec<_> = lex(line, Mode::Module).collect();
        let indexer = Indexer::from_tokens(&tokens, &locator);
        let stylist = Stylist::from_tokens(&tokens, &locator);

        let check_with_max_line_length = |line_length: LineLength| {
            check_physical_lines(
                &locator,
                &stylist,
                &indexer,
                &[],
                &Settings {
                    line_length,
                    ..Settings::for_rule(Rule::LineTooLong)
                },
            )
        };
        let line_length = LineLength::from(8);
        assert_eq!(check_with_max_line_length(line_length), vec![]);
        assert_eq!(check_with_max_line_length(line_length), vec![]);
    }
}
