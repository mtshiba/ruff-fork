use std::borrow::Cow;
use std::cell::LazyCell;
use std::ops::Deref;
use std::path::Path;

use anyhow::{anyhow, Result};
use colored::Colorize;
use itertools::Itertools;
use rustc_hash::FxHashMap;

use ruff_diagnostics::Diagnostic;
use ruff_notebook::Notebook;
use ruff_python_ast::{ModModule, PySourceType, PythonVersion};
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::{ParseError, ParseOptions, Parsed, UnsupportedSyntaxError};
use ruff_source_file::SourceFileBuilder;
use ruff_text_size::Ranged;

use crate::checkers::ast::check_ast;
use crate::checkers::filesystem::check_file_path;
use crate::checkers::imports::check_imports;
use crate::checkers::noqa::check_noqa;
use crate::checkers::physical_lines::check_physical_lines;
use crate::checkers::tokens::check_tokens;
use crate::directives::Directives;
use crate::doc_lines::{doc_lines_from_ast, doc_lines_from_tokens};
use crate::fix::{fix_file, FixResult};
use crate::message::Message;
use crate::noqa::add_noqa;
use crate::package::PackageRoot;
use crate::registry::{AsRule, Rule, RuleSet};
#[cfg(any(feature = "test-rules", test))]
use crate::rules::ruff::rules::test_rules::{self, TestRule, TEST_RULES};
use crate::settings::types::UnsafeFixes;
use crate::settings::{flags, LinterSettings};
use crate::source_kind::SourceKind;
use crate::{directives, fs, Locator};

pub struct LinterResult {
    /// A collection of diagnostic messages generated by the linter.
    pub messages: Vec<Message>,
    /// A flag indicating the presence of syntax errors in the source file.
    /// If `true`, at least one syntax error was detected in the source file.
    ///
    /// This includes both [`ParseError`]s and [`UnsupportedSyntaxError`]s.
    pub has_syntax_error: bool,
}

pub type FixTable = FxHashMap<Rule, usize>;

pub struct FixerResult<'a> {
    /// The result returned by the linter, after applying any fixes.
    pub result: LinterResult,
    /// The resulting source code, after applying any fixes.
    pub transformed: Cow<'a, SourceKind>,
    /// The number of fixes applied for each [`Rule`].
    pub fixed: FixTable,
}

/// Generate `Diagnostic`s from the source code contents at the
/// given `Path`.
#[allow(clippy::too_many_arguments)]
pub fn check_path(
    path: &Path,
    package: Option<PackageRoot<'_>>,
    locator: &Locator,
    stylist: &Stylist,
    indexer: &Indexer,
    directives: &Directives,
    settings: &LinterSettings,
    noqa: flags::Noqa,
    source_kind: &SourceKind,
    source_type: PySourceType,
    parsed: &Parsed<ModModule>,
    target_version: PythonVersion,
) -> Vec<Diagnostic> {
    // Aggregate all diagnostics.
    let mut diagnostics = vec![];

    let tokens = parsed.tokens();
    let comment_ranges = indexer.comment_ranges();

    // Collect doc lines. This requires a rare mix of tokens (for comments) and AST
    // (for docstrings), which demands special-casing at this level.
    let use_doc_lines = settings.rules.enabled(Rule::DocLineTooLong);
    let mut doc_lines = vec![];
    if use_doc_lines {
        doc_lines.extend(doc_lines_from_tokens(tokens));
    }

    // Run the token-based rules.
    if settings
        .rules
        .iter_enabled()
        .any(|rule_code| rule_code.lint_source().is_tokens())
    {
        diagnostics.extend(check_tokens(
            tokens,
            path,
            locator,
            indexer,
            stylist,
            settings,
            source_type,
            source_kind.as_ipy_notebook().map(Notebook::cell_offsets),
        ));
    }

    // Run the filesystem-based rules.
    if settings
        .rules
        .iter_enabled()
        .any(|rule_code| rule_code.lint_source().is_filesystem())
    {
        diagnostics.extend(check_file_path(
            path,
            package,
            locator,
            comment_ranges,
            settings,
            target_version,
        ));
    }

    // Run the logical line-based rules.
    if settings
        .rules
        .iter_enabled()
        .any(|rule_code| rule_code.lint_source().is_logical_lines())
    {
        diagnostics.extend(crate::checkers::logical_lines::check_logical_lines(
            tokens, locator, indexer, stylist, settings,
        ));
    }

    // Run the AST-based rules only if there are no syntax errors.
    if parsed.has_valid_syntax() {
        let use_ast = settings
            .rules
            .iter_enabled()
            .any(|rule_code| rule_code.lint_source().is_ast());
        let use_imports = !directives.isort.skip_file
            && settings
                .rules
                .iter_enabled()
                .any(|rule_code| rule_code.lint_source().is_imports());
        if use_ast || use_imports || use_doc_lines {
            let cell_offsets = source_kind.as_ipy_notebook().map(Notebook::cell_offsets);
            let notebook_index = source_kind.as_ipy_notebook().map(Notebook::index);
            if use_ast {
                diagnostics.extend(check_ast(
                    parsed,
                    locator,
                    stylist,
                    indexer,
                    &directives.noqa_line_for,
                    settings,
                    noqa,
                    path,
                    package,
                    source_type,
                    cell_offsets,
                    notebook_index,
                    target_version,
                ));
            }
            if use_imports {
                let import_diagnostics = check_imports(
                    parsed,
                    locator,
                    indexer,
                    &directives.isort,
                    settings,
                    stylist,
                    package,
                    source_type,
                    cell_offsets,
                    target_version,
                );

                diagnostics.extend(import_diagnostics);
            }
            if use_doc_lines {
                doc_lines.extend(doc_lines_from_ast(parsed.suite(), locator));
            }
        }
    }

    // Deduplicate and reorder any doc lines.
    if use_doc_lines {
        doc_lines.sort_unstable();
        doc_lines.dedup();
    }

    // Run the lines-based rules.
    if settings
        .rules
        .iter_enabled()
        .any(|rule_code| rule_code.lint_source().is_physical_lines())
    {
        diagnostics.extend(check_physical_lines(
            locator, stylist, indexer, &doc_lines, settings,
        ));
    }

    // Raise violations for internal test rules
    #[cfg(any(feature = "test-rules", test))]
    {
        for test_rule in TEST_RULES {
            if !settings.rules.enabled(*test_rule) {
                continue;
            }
            let diagnostic = match test_rule {
                Rule::StableTestRule => {
                    test_rules::StableTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::StableTestRuleSafeFix => {
                    test_rules::StableTestRuleSafeFix::diagnostic(locator, comment_ranges)
                }
                Rule::StableTestRuleUnsafeFix => {
                    test_rules::StableTestRuleUnsafeFix::diagnostic(locator, comment_ranges)
                }
                Rule::StableTestRuleDisplayOnlyFix => {
                    test_rules::StableTestRuleDisplayOnlyFix::diagnostic(locator, comment_ranges)
                }
                Rule::PreviewTestRule => {
                    test_rules::PreviewTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::DeprecatedTestRule => {
                    test_rules::DeprecatedTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::AnotherDeprecatedTestRule => {
                    test_rules::AnotherDeprecatedTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::RemovedTestRule => {
                    test_rules::RemovedTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::AnotherRemovedTestRule => {
                    test_rules::AnotherRemovedTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::RedirectedToTestRule => {
                    test_rules::RedirectedToTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::RedirectedFromTestRule => {
                    test_rules::RedirectedFromTestRule::diagnostic(locator, comment_ranges)
                }
                Rule::RedirectedFromPrefixTestRule => {
                    test_rules::RedirectedFromPrefixTestRule::diagnostic(locator, comment_ranges)
                }
                _ => unreachable!("All test rules must have an implementation"),
            };
            if let Some(diagnostic) = diagnostic {
                diagnostics.push(diagnostic);
            }
        }
    }

    // Ignore diagnostics based on per-file-ignores.
    let per_file_ignores = if (!diagnostics.is_empty()
        || settings
            .rules
            .iter_enabled()
            .any(|rule_code| rule_code.lint_source().is_noqa()))
        && !settings.per_file_ignores.is_empty()
    {
        fs::ignores_from_path(path, &settings.per_file_ignores)
    } else {
        RuleSet::empty()
    };
    if !per_file_ignores.is_empty() {
        diagnostics.retain(|diagnostic| !per_file_ignores.contains(diagnostic.kind.rule()));
    }

    // Enforce `noqa` directives.
    if noqa.is_enabled()
        || settings
            .rules
            .iter_enabled()
            .any(|rule_code| rule_code.lint_source().is_noqa())
    {
        let ignored = check_noqa(
            &mut diagnostics,
            path,
            locator,
            comment_ranges,
            &directives.noqa_line_for,
            parsed.has_valid_syntax(),
            &per_file_ignores,
            settings,
        );
        if noqa.is_enabled() {
            for index in ignored.iter().rev() {
                diagnostics.swap_remove(*index);
            }
        }
    }

    if parsed.has_valid_syntax() {
        // Remove fixes for any rules marked as unfixable.
        for diagnostic in &mut diagnostics {
            if !settings.rules.should_fix(diagnostic.kind.rule()) {
                diagnostic.fix = None;
            }
        }

        // Update fix applicability to account for overrides
        if !settings.fix_safety.is_empty() {
            for diagnostic in &mut diagnostics {
                if let Some(fix) = diagnostic.fix.take() {
                    let fixed_applicability = settings
                        .fix_safety
                        .resolve_applicability(diagnostic.kind.rule(), fix.applicability());
                    diagnostic.set_fix(fix.with_applicability(fixed_applicability));
                }
            }
        }
    } else {
        // Avoid fixing in case the source code contains syntax errors.
        for diagnostic in &mut diagnostics {
            diagnostic.fix = None;
        }
    }

    diagnostics
}

const MAX_ITERATIONS: usize = 100;

/// Add any missing `# noqa` pragmas to the source code at the given `Path`.
pub fn add_noqa_to_path(
    path: &Path,
    package: Option<PackageRoot<'_>>,
    source_kind: &SourceKind,
    source_type: PySourceType,
    settings: &LinterSettings,
) -> Result<usize> {
    // Parse once.
    let target_version = settings.resolve_target_version(path);
    let parsed = parse_unchecked_source(source_kind, source_type, target_version);

    // Map row and column locations to byte slices (lazily).
    let locator = Locator::new(source_kind.source_code());

    // Detect the current code style (lazily).
    let stylist = Stylist::from_tokens(parsed.tokens(), locator.contents());

    // Extra indices from the code.
    let indexer = Indexer::from_tokens(parsed.tokens(), locator.contents());

    // Extract the `# noqa` and `# isort: skip` directives from the source.
    let directives = directives::extract_directives(
        parsed.tokens(),
        directives::Flags::from_settings(settings),
        &locator,
        &indexer,
    );

    // Generate diagnostics, ignoring any existing `noqa` directives.
    let diagnostics = check_path(
        path,
        package,
        &locator,
        &stylist,
        &indexer,
        &directives,
        settings,
        flags::Noqa::Disabled,
        source_kind,
        source_type,
        &parsed,
        target_version,
    );

    // Add any missing `# noqa` pragmas.
    // TODO(dhruvmanila): Add support for Jupyter Notebooks
    add_noqa(
        path,
        &diagnostics,
        &locator,
        indexer.comment_ranges(),
        &settings.external,
        &directives.noqa_line_for,
        stylist.line_ending(),
    )
}

/// Generate a [`Message`] for each [`Diagnostic`] triggered by the given source
/// code.
pub fn lint_only(
    path: &Path,
    package: Option<PackageRoot<'_>>,
    settings: &LinterSettings,
    noqa: flags::Noqa,
    source_kind: &SourceKind,
    source_type: PySourceType,
    source: ParseSource,
) -> LinterResult {
    let target_version = settings.resolve_target_version(path);
    let parsed = source.into_parsed(source_kind, source_type, target_version);

    // Map row and column locations to byte slices (lazily).
    let locator = Locator::new(source_kind.source_code());

    // Detect the current code style (lazily).
    let stylist = Stylist::from_tokens(parsed.tokens(), locator.contents());

    // Extra indices from the code.
    let indexer = Indexer::from_tokens(parsed.tokens(), locator.contents());

    // Extract the `# noqa` and `# isort: skip` directives from the source.
    let directives = directives::extract_directives(
        parsed.tokens(),
        directives::Flags::from_settings(settings),
        &locator,
        &indexer,
    );

    // Generate diagnostics.
    let diagnostics = check_path(
        path,
        package,
        &locator,
        &stylist,
        &indexer,
        &directives,
        settings,
        noqa,
        source_kind,
        source_type,
        &parsed,
        target_version,
    );

    let syntax_errors = if settings.preview.is_enabled() {
        parsed.unsupported_syntax_errors()
    } else {
        &[]
    };

    LinterResult {
        messages: diagnostics_to_messages(
            diagnostics,
            parsed.errors(),
            syntax_errors,
            path,
            &locator,
            &directives,
        ),
        has_syntax_error: !parsed.has_no_syntax_errors(),
    }
}

/// Convert from diagnostics to messages.
fn diagnostics_to_messages(
    diagnostics: Vec<Diagnostic>,
    parse_errors: &[ParseError],
    unsupported_syntax_errors: &[UnsupportedSyntaxError],
    path: &Path,
    locator: &Locator,
    directives: &Directives,
) -> Vec<Message> {
    let file = LazyCell::new(|| {
        let mut builder =
            SourceFileBuilder::new(path.to_string_lossy().as_ref(), locator.contents());

        if let Some(line_index) = locator.line_index() {
            builder.set_line_index(line_index.clone());
        }

        builder.finish()
    });

    parse_errors
        .iter()
        .map(|parse_error| Message::from_parse_error(parse_error, locator, file.deref().clone()))
        .chain(unsupported_syntax_errors.iter().map(|syntax_error| {
            Message::from_unsupported_syntax_error(syntax_error, file.deref().clone())
        }))
        .chain(diagnostics.into_iter().map(|diagnostic| {
            let noqa_offset = directives.noqa_line_for.resolve(diagnostic.start());
            Message::from_diagnostic(diagnostic, file.deref().clone(), noqa_offset)
        }))
        .collect()
}

/// Generate `Diagnostic`s from source code content, iteratively fixing
/// until stable.
#[allow(clippy::too_many_arguments)]
pub fn lint_fix<'a>(
    path: &Path,
    package: Option<PackageRoot<'_>>,
    noqa: flags::Noqa,
    unsafe_fixes: UnsafeFixes,
    settings: &LinterSettings,
    source_kind: &'a SourceKind,
    source_type: PySourceType,
) -> Result<FixerResult<'a>> {
    let mut transformed = Cow::Borrowed(source_kind);

    // Track the number of fixed errors across iterations.
    let mut fixed = FxHashMap::default();

    // As an escape hatch, bail after 100 iterations.
    let mut iterations = 0;

    // Track whether the _initial_ source code is valid syntax.
    let mut is_valid_syntax = false;

    let target_version = settings.resolve_target_version(path);

    // Continuously fix until the source code stabilizes.
    loop {
        // Parse once.
        let parsed = parse_unchecked_source(&transformed, source_type, target_version);

        // Map row and column locations to byte slices (lazily).
        let locator = Locator::new(transformed.source_code());

        // Detect the current code style (lazily).
        let stylist = Stylist::from_tokens(parsed.tokens(), locator.contents());

        // Extra indices from the code.
        let indexer = Indexer::from_tokens(parsed.tokens(), locator.contents());

        // Extract the `# noqa` and `# isort: skip` directives from the source.
        let directives = directives::extract_directives(
            parsed.tokens(),
            directives::Flags::from_settings(settings),
            &locator,
            &indexer,
        );

        // Generate diagnostics.
        let diagnostics = check_path(
            path,
            package,
            &locator,
            &stylist,
            &indexer,
            &directives,
            settings,
            noqa,
            &transformed,
            source_type,
            &parsed,
            target_version,
        );

        if iterations == 0 {
            is_valid_syntax = parsed.has_no_syntax_errors();
        } else {
            // If the source code was parseable on the first pass, but is no
            // longer parseable on a subsequent pass, then we've introduced a
            // syntax error. Return the original code.
            if is_valid_syntax {
                if let Some(error) = parsed.errors().first() {
                    report_fix_syntax_error(
                        path,
                        transformed.source_code(),
                        error,
                        fixed.keys().copied(),
                    );
                    return Err(anyhow!("Fix introduced a syntax error"));
                }
            }
        }

        // Apply fix.
        if let Some(FixResult {
            code: fixed_contents,
            fixes: applied,
            source_map,
        }) = fix_file(&diagnostics, &locator, unsafe_fixes)
        {
            if iterations < MAX_ITERATIONS {
                // Count the number of fixed errors.
                for (rule, count) in applied {
                    *fixed.entry(rule).or_default() += count;
                }

                transformed = Cow::Owned(transformed.updated(fixed_contents, &source_map));

                // Increment the iteration count.
                iterations += 1;

                // Re-run the linter pass (by avoiding the return).
                continue;
            }

            report_failed_to_converge_error(path, transformed.source_code(), &diagnostics);
        }

        let syntax_errors = if settings.preview.is_enabled() {
            parsed.unsupported_syntax_errors()
        } else {
            &[]
        };

        return Ok(FixerResult {
            result: LinterResult {
                messages: diagnostics_to_messages(
                    diagnostics,
                    parsed.errors(),
                    syntax_errors,
                    path,
                    &locator,
                    &directives,
                ),
                has_syntax_error: !is_valid_syntax,
            },
            transformed,
            fixed,
        });
    }
}

fn collect_rule_codes(rules: impl IntoIterator<Item = Rule>) -> String {
    rules
        .into_iter()
        .map(|rule| rule.noqa_code().to_string())
        .sorted_unstable()
        .dedup()
        .join(", ")
}

#[allow(clippy::print_stderr)]
fn report_failed_to_converge_error(path: &Path, transformed: &str, diagnostics: &[Diagnostic]) {
    let codes = collect_rule_codes(diagnostics.iter().map(|diagnostic| diagnostic.kind.rule()));
    if cfg!(debug_assertions) {
        eprintln!(
            "{}{} Failed to converge after {} iterations in `{}` with rule codes {}:---\n{}\n---",
            "debug error".red().bold(),
            ":".bold(),
            MAX_ITERATIONS,
            fs::relativize_path(path),
            codes,
            transformed,
        );
    } else {
        eprintln!(
            r#"
{}{} Failed to converge after {} iterations.

This indicates a bug in Ruff. If you could open an issue at:

    https://github.com/astral-sh/ruff/issues/new?title=%5BInfinite%20loop%5D

...quoting the contents of `{}`, the rule codes {}, along with the `pyproject.toml` settings and executed command, we'd be very appreciative!
"#,
            "error".red().bold(),
            ":".bold(),
            MAX_ITERATIONS,
            fs::relativize_path(path),
            codes
        );
    }
}

#[allow(clippy::print_stderr)]
fn report_fix_syntax_error(
    path: &Path,
    transformed: &str,
    error: &ParseError,
    rules: impl IntoIterator<Item = Rule>,
) {
    let codes = collect_rule_codes(rules);
    if cfg!(debug_assertions) {
        eprintln!(
            "{}{} Fix introduced a syntax error in `{}` with rule codes {}: {}\n---\n{}\n---",
            "error".red().bold(),
            ":".bold(),
            fs::relativize_path(path),
            codes,
            error,
            transformed,
        );
    } else {
        eprintln!(
            r#"
{}{} Fix introduced a syntax error. Reverting all changes.

This indicates a bug in Ruff. If you could open an issue at:

    https://github.com/astral-sh/ruff/issues/new?title=%5BFix%20error%5D

...quoting the contents of `{}`, the rule codes {}, along with the `pyproject.toml` settings and executed command, we'd be very appreciative!
"#,
            "error".red().bold(),
            ":".bold(),
            fs::relativize_path(path),
            codes,
        );
    }
}

#[derive(Debug, Clone)]
pub enum ParseSource {
    /// Parse the [`Parsed`] from the given source code.
    None,
    /// Use the precomputed [`Parsed`].
    Precomputed(Parsed<ModModule>),
}

impl ParseSource {
    /// Consumes the [`ParseSource`] and returns the parsed [`Parsed`], parsing the source code if
    /// necessary.
    fn into_parsed(
        self,
        source_kind: &SourceKind,
        source_type: PySourceType,
        target_version: PythonVersion,
    ) -> Parsed<ModModule> {
        match self {
            ParseSource::None => parse_unchecked_source(source_kind, source_type, target_version),
            ParseSource::Precomputed(parsed) => parsed,
        }
    }
}

/// Like [`ruff_python_parser::parse_unchecked_source`] but with an additional [`PythonVersion`]
/// argument.
fn parse_unchecked_source(
    source_kind: &SourceKind,
    source_type: PySourceType,
    target_version: PythonVersion,
) -> Parsed<ModModule> {
    let options = ParseOptions::from(source_type).with_target_version(target_version);
    // SAFETY: Safe because `PySourceType` always parses to a `ModModule`. See
    // `ruff_python_parser::parse_unchecked_source`. We use `parse_unchecked` (and thus
    // have to unwrap) in order to pass the `PythonVersion` via `ParseOptions`.
    ruff_python_parser::parse_unchecked(source_kind.source_code(), options)
        .try_into_module()
        .expect("PySourceType always parses into a module")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use test_case::test_case;

    use ruff_notebook::{Notebook, NotebookError};

    use crate::registry::Rule;
    use crate::source_kind::SourceKind;
    use crate::test::{assert_notebook_path, test_contents, TestedNotebook};
    use crate::{assert_messages, settings};

    /// Construct a path to a Jupyter notebook in the `resources/test/fixtures/jupyter` directory.
    fn notebook_path(path: impl AsRef<Path>) -> std::path::PathBuf {
        Path::new("../ruff_notebook/resources/test/fixtures/jupyter").join(path)
    }

    #[test]
    fn test_import_sorting() -> Result<(), NotebookError> {
        let actual = notebook_path("isort.ipynb");
        let expected = notebook_path("isort_expected.ipynb");
        let TestedNotebook {
            messages,
            source_notebook,
            ..
        } = assert_notebook_path(
            &actual,
            expected,
            &settings::LinterSettings::for_rule(Rule::UnsortedImports),
        )?;
        assert_messages!(messages, actual, source_notebook);
        Ok(())
    }

    #[test]
    fn test_ipy_escape_command() -> Result<(), NotebookError> {
        let actual = notebook_path("ipy_escape_command.ipynb");
        let expected = notebook_path("ipy_escape_command_expected.ipynb");
        let TestedNotebook {
            messages,
            source_notebook,
            ..
        } = assert_notebook_path(
            &actual,
            expected,
            &settings::LinterSettings::for_rule(Rule::UnusedImport),
        )?;
        assert_messages!(messages, actual, source_notebook);
        Ok(())
    }

    #[test]
    fn test_unused_variable() -> Result<(), NotebookError> {
        let actual = notebook_path("unused_variable.ipynb");
        let expected = notebook_path("unused_variable_expected.ipynb");
        let TestedNotebook {
            messages,
            source_notebook,
            ..
        } = assert_notebook_path(
            &actual,
            expected,
            &settings::LinterSettings::for_rule(Rule::UnusedVariable),
        )?;
        assert_messages!(messages, actual, source_notebook);
        Ok(())
    }

    #[test]
    fn test_undefined_name() -> Result<(), NotebookError> {
        let actual = notebook_path("undefined_name.ipynb");
        let expected = notebook_path("undefined_name.ipynb");
        let TestedNotebook {
            messages,
            source_notebook,
            ..
        } = assert_notebook_path(
            &actual,
            expected,
            &settings::LinterSettings::for_rule(Rule::UndefinedName),
        )?;
        assert_messages!(messages, actual, source_notebook);
        Ok(())
    }

    #[test]
    fn test_json_consistency() -> Result<()> {
        let actual_path = notebook_path("before_fix.ipynb");
        let expected_path = notebook_path("after_fix.ipynb");

        let TestedNotebook {
            linted_notebook: fixed_notebook,
            ..
        } = assert_notebook_path(
            actual_path,
            &expected_path,
            &settings::LinterSettings::for_rule(Rule::UnusedImport),
        )?;
        let mut writer = Vec::new();
        fixed_notebook.write(&mut writer)?;
        let actual = String::from_utf8(writer)?;
        let expected = std::fs::read_to_string(expected_path)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_vscode_language_id() -> Result<()> {
        let actual = notebook_path("vscode_language_id.ipynb");
        let expected = notebook_path("vscode_language_id_expected.ipynb");
        let TestedNotebook {
            messages,
            source_notebook,
            ..
        } = assert_notebook_path(
            &actual,
            expected,
            &settings::LinterSettings::for_rule(Rule::UnusedImport),
        )?;
        assert_messages!(messages, actual, source_notebook);
        Ok(())
    }

    #[test_case(Path::new("before_fix.ipynb"), true; "trailing_newline")]
    #[test_case(Path::new("no_trailing_newline.ipynb"), false; "no_trailing_newline")]
    fn test_trailing_newline(path: &Path, trailing_newline: bool) -> Result<()> {
        let notebook = Notebook::from_path(&notebook_path(path))?;
        assert_eq!(notebook.trailing_newline(), trailing_newline);

        let mut writer = Vec::new();
        notebook.write(&mut writer)?;
        let string = String::from_utf8(writer)?;
        assert_eq!(string.ends_with('\n'), trailing_newline);

        Ok(())
    }

    // Version <4.5, don't emit cell ids
    #[test_case(Path::new("no_cell_id.ipynb"), false; "no_cell_id")]
    // Version 4.5, cell ids are missing and need to be added
    #[test_case(Path::new("add_missing_cell_id.ipynb"), true; "add_missing_cell_id")]
    fn test_cell_id(path: &Path, has_id: bool) -> Result<()> {
        let source_notebook = Notebook::from_path(&notebook_path(path))?;
        let source_kind = SourceKind::IpyNotebook(source_notebook);
        let (_, transformed) = test_contents(
            &source_kind,
            path,
            &settings::LinterSettings::for_rule(Rule::UnusedImport),
        );
        let linted_notebook = transformed.into_owned().expect_ipy_notebook();
        let mut writer = Vec::new();
        linted_notebook.write(&mut writer)?;
        let actual = String::from_utf8(writer)?;
        if has_id {
            assert!(actual.contains(r#""id": ""#));
        } else {
            assert!(!actual.contains(r#""id":"#));
        }
        Ok(())
    }
}
