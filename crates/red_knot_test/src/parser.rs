use memchr::memchr2;
use regex::{Captures, Match, Regex};
use ruff_index::{newtype_index, IndexVec};
use ruff_source_file::{LineIndex, OneIndexed};
use ruff_text_size::TextSize;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::LazyLock;

/// Parse the Markdown `source` as a test suite with given `title`.
pub(crate) fn parse<'s>(title: &'s str, source: &'s str) -> anyhow::Result<MarkdownTestSuite<'s>> {
    let parser = Parser::new(title, source);
    parser.parse()
}

/// A parsed markdown file containing tests.
///
/// Borrows from the source string and filepath it was created from.
#[derive(Debug)]
pub(crate) struct MarkdownTestSuite<'s> {
    /// Header sections.
    sections: IndexVec<SectionId, Section<'s>>,

    /// Test files embedded within the Markdown file.
    files: IndexVec<EmbeddedFileId, EmbeddedFile<'s>>,
}

impl<'s> MarkdownTestSuite<'s> {
    pub(crate) fn tests(&self) -> MarkdownTestIterator<'_, 's> {
        MarkdownTestIterator {
            suite: self,
            current_file_index: 0,
        }
    }
}

/// A single test inside a [`MarkdownTestSuite`].
///
/// A test is a single header section (or the implicit root section, if there are no Markdown
/// headers in the file), containing one or more embedded Python files as fenced code blocks, and
/// containing no nested header subsections.
#[derive(Debug)]
pub(crate) struct MarkdownTest<'m, 's> {
    suite: &'m MarkdownTestSuite<'s>,
    section: &'m Section<'s>,
    files: &'m [EmbeddedFile<'s>],
}

impl<'m, 's> MarkdownTest<'m, 's> {
    pub(crate) fn name(&self) -> String {
        let mut name = String::new();
        let mut parent_id = self.section.parent_id;
        while let Some(next_id) = parent_id {
            let parent = &self.suite.sections[next_id];
            parent_id = parent.parent_id;
            if !name.is_empty() {
                name.insert_str(0, " - ");
            }
            name.insert_str(0, parent.title);
        }
        if !name.is_empty() {
            name.push_str(" - ");
        }
        name.push_str(self.section.title);
        name
    }

    pub(crate) fn files(&self) -> impl Iterator<Item = &'m EmbeddedFile<'s>> {
        self.files.iter()
    }
}

/// Iterator yielding all [`MarkdownTest`]s in a [`MarkdownTestSuite`].
#[derive(Debug)]
pub(crate) struct MarkdownTestIterator<'m, 's> {
    suite: &'m MarkdownTestSuite<'s>,
    current_file_index: usize,
}

impl<'m, 's> Iterator for MarkdownTestIterator<'m, 's> {
    type Item = MarkdownTest<'m, 's>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current_file_index = self.current_file_index;
        let mut file = self.suite.files.get(current_file_index.into());
        let section_id = file?.section;
        while file.is_some_and(|file| file.section == section_id) {
            current_file_index += 1;
            file = self.suite.files.get(current_file_index.into());
        }
        let files = &self.suite.files[EmbeddedFileId::from_usize(self.current_file_index)
            ..EmbeddedFileId::from_usize(current_file_index)];
        self.current_file_index = current_file_index;
        Some(MarkdownTest {
            suite: self.suite,
            section: &self.suite.sections[section_id],
            files,
        })
    }
}

#[newtype_index]
struct SectionId;

/// A single header section of a [`MarkdownTestSuite`], or the implicit root "section".
///
/// A header section is the part of a Markdown file beginning with a `#`-prefixed header line, and
/// extending until the next header line at the same or higher outline level (that is, with the
/// same number or fewer `#` characters).
///
/// A header section may either contain one or more embedded Python files (making it a
/// [`MarkdownTest`]), or it may contain nested sections (headers with more `#` characters), but
/// not both.
#[derive(Debug)]
struct Section<'s> {
    title: &'s str,
    level: u8,
    parent_id: Option<SectionId>,
}

#[newtype_index]
struct EmbeddedFileId;

/// A single file embedded in a [`Section`] as a fenced code block.
///
/// Currently must be a Python file (`py` language) or type stub (`pyi`). In the future we plan
/// support other kinds of files as well (TOML configuration, typeshed VERSIONS, `pth` files...).
///
/// A Python embedded file makes its containing [`Section`] into a [`MarkdownTest`], and will be
/// type-checked and searched for inline-comment assertions to match against the diagnostics from
/// type checking.
#[derive(Debug)]
pub(crate) struct EmbeddedFile<'s> {
    section: SectionId,
    pub(crate) path: &'s str,
    pub(crate) lang: &'s str,
    pub(crate) code: &'s str,

    /// The line number of the ``` in the markdown file
    pub(crate) starting_line_number: OneIndexed,
}

/// Matches a sequence of `#` characters, followed by a title heading, followed by a newline.
static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<level>#+)\s+(?<title>.+)\s*\n").unwrap());

/// Matches a code block fenced by triple backticks, possibly with language and `key=val`
/// configuration items following the opening backticks (in the "tag string" of the code block).
static CODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^```(?<lang>(?-u:\w)+)?(?<config>(?: +\S+)*)\s*\n(?<code>(?:.|\n)*?)\n?```\s*\n")
        .unwrap()
});

#[derive(Debug)]
struct SectionStack(Vec<SectionId>);

impl SectionStack {
    fn new(root_section_id: SectionId) -> Self {
        Self(vec![root_section_id])
    }

    fn push(&mut self, section_id: SectionId) {
        self.0.push(section_id);
    }

    fn pop(&mut self) -> Option<SectionId> {
        let popped = self.0.pop();
        debug_assert_ne!(popped, None, "Should never pop the implicit root section");
        debug_assert!(
            !self.0.is_empty(),
            "Should never pop the implicit root section"
        );
        popped
    }

    fn parent(&mut self) -> SectionId {
        *self
            .0
            .last()
            .expect("Should never pop the implicit root section")
    }
}

/// Parse the source of a Markdown file into a [`MarkdownTestSuite`].
#[derive(Debug)]
struct Parser<'s> {
    /// [`Section`]s of the final [`MarkdownTestSuite`].
    sections: IndexVec<SectionId, Section<'s>>,

    /// [`EmbeddedFile`]s of the final [`MarkdownTestSuite`].
    files: IndexVec<EmbeddedFileId, EmbeddedFile<'s>>,

    /// The unparsed remainder of the Markdown source.
    unparsed: &'s str,

    /// Current offset of the parser into the markdown file.
    md_offset: TextSize,

    line_index: LineIndex,

    /// Stack of ancestor sections.
    stack: SectionStack,

    /// Names of embedded files in current active section.
    current_section_files: Option<FxHashSet<&'s str>>,
}

impl<'s> Parser<'s> {
    fn new(title: &'s str, source: &'s str) -> Self {
        let mut sections = IndexVec::default();
        let root_section_id = sections.push(Section {
            title,
            level: 0,
            parent_id: None,
        });
        Self {
            sections,
            files: IndexVec::default(),
            unparsed: source,
            md_offset: TextSize::new(0),
            line_index: LineIndex::from_source_text(&source),
            stack: SectionStack::new(root_section_id),
            current_section_files: None,
        }
    }

    fn parse(mut self) -> anyhow::Result<MarkdownTestSuite<'s>> {
        self.parse_impl()?;
        Ok(self.finish())
    }

    fn finish(mut self) -> MarkdownTestSuite<'s> {
        self.sections.shrink_to_fit();
        self.files.shrink_to_fit();

        MarkdownTestSuite {
            sections: self.sections,
            files: self.files,
        }
    }

    fn increment_offset(&mut self, size: usize) -> anyhow::Result<()> {
        self.md_offset = self
            .md_offset
            .checked_add(size.try_into()?)
            .ok_or_else(|| anyhow::anyhow!("Overflow when incrementing offset by {size}"))?;

        Ok(())
    }

    fn parse_impl(&mut self) -> anyhow::Result<()> {
        while let Some(position) = memchr2(b'`', b'#', self.unparsed.as_bytes()) {
            let (before, after) = self.unparsed.split_at(position);
            self.unparsed = after;

            // code blocks and headers must start on a new line.
            if before.is_empty() || before.ends_with('\n') {
                let c = after.as_bytes()[0] as char;

                match c {
                    '#' => {
                        if let Some(find) = HEADER_RE.find(self.unparsed) {
                            self.parse_header(find.as_str())?;
                            self.unparsed = &self.unparsed[find.end()..];
                            continue;
                        }
                    }
                    '`' => {
                        if let Some(captures) = CODE_RE.captures(self.unparsed) {
                            self.parse_code_block(&captures)?;
                            self.unparsed = &self.unparsed[captures.get(0).unwrap().end()..];
                            continue;
                        }
                    }
                    _ => unreachable!(),
                }
            }

            // Skip to the end of the line
            if let Some(position) = memchr::memchr(b'\n', self.unparsed.as_bytes()) {
                self.unparsed = &self.unparsed[position + 1..];
            } else {
                break;
            }
        }

        Ok(())
    }

    fn parse_header(&mut self, header: &'s str) -> anyhow::Result<()> {
        let mut trimmed = header.trim();

        let mut header_level = 0usize;
        while let Some(rest) = trimmed.strip_prefix('#') {
            header_level += 1;
            trimmed = rest;
        }

        let title = trimmed.trim_start();

        self.pop_sections_to_level(header_level);

        let parent = self.stack.parent();

        let section = Section {
            title,
            level: header_level.try_into()?,
            parent_id: Some(parent),
        };

        if self.current_section_files.is_some() {
            return Err(anyhow::anyhow!(
                "Header '{}' not valid inside a test case; parent '{}' has code files.",
                section.title,
                self.sections[parent].title,
            ));
        }

        let section_id = self.sections.push(section);
        self.stack.push(section_id);

        self.current_section_files = None;

        self.increment_offset(
            captures
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("No captures found"))?
                .len(),
        )?;

        Ok(())
    }

    fn parse_code_block(&mut self, captures: &Captures<'s>) -> anyhow::Result<()> {
        // We never pop the implicit root section.
        let parent = self.stack.parent();

        let mut config: FxHashMap<&'s str, &'s str> = FxHashMap::default();

        if let Some(config_match) = captures.name("config") {
            for item in config_match.as_str().split_whitespace() {
                let mut parts = item.split('=');
                let key = parts.next().unwrap();
                let Some(val) = parts.next() else {
                    return Err(anyhow::anyhow!("Invalid config item `{}`.", item));
                };
                if parts.next().is_some() {
                    return Err(anyhow::anyhow!("Invalid config item `{}`.", item));
                }
                if config.insert(key, val).is_some() {
                    return Err(anyhow::anyhow!("Duplicate config item `{}`.", item));
                }
            }
        }

        let path = config.get("path").copied().unwrap_or("test.py");

        self.files.push(EmbeddedFile {
            path,
            section: parent,
            lang: captures
                .name("lang")
                .as_ref()
                .map(Match::as_str)
                .unwrap_or_default(),
            // CODE_RE can't match without matches for 'lang' and 'code'.
            code: captures.name("code").unwrap().into(),

            starting_line_number: self.line_index.line_index(self.md_offset),
        });

        if let Some(current_files) = &mut self.current_section_files {
            if !current_files.insert(path) {
                if path == "test.py" {
                    return Err(anyhow::anyhow!(
                        "Test `{}` has duplicate files named `{path}`. \
                                (This is the default filename; \
                                 consider giving some files an explicit name with `path=...`.)",
                        self.sections[parent].title
                    ));
                }
                return Err(anyhow::anyhow!(
                    "Test `{}` has duplicate files named `{path}`.",
                    self.sections[parent].title
                ));
            };
        } else {
            self.current_section_files = Some(FxHashSet::from_iter([path]));
        }

        self.increment_offset(
            captures
                .get(0)
                .ok_or_else(|| anyhow::anyhow!("No captures found"))?
                .len(),
        )?;

        Ok(())
    }

    fn pop_sections_to_level(&mut self, level: usize) {
        while level <= self.sections[self.stack.parent()].level.into() {
            self.stack.pop();
            // We would have errored before pushing a child section if there were files, so we know
            // no parent section can have files.
            self.current_section_files = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_trivia::textwrap::dedent;

    #[test]
    fn empty() {
        let mf = super::parse("file.md", "").unwrap();

        assert!(mf.tests().next().is_none());
    }

    #[test]
    fn single_file_test() {
        let source = dedent(
            "
            ```py
            x = 1
            ```
            ",
        );
        let mf = super::parse("file.md", &source).unwrap();

        let [test] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected one test");
        };

        assert_eq!(test.name(), "file.md");

        let [file] = test.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.path, "test.py");
        assert_eq!(file.lang, "py");
        assert_eq!(file.code, "x = 1");
    }

    #[test]
    fn multiple_tests() {
        let source = dedent(
            "
            # One

            ```py
            x = 1
            ```

            # Two

            ```py
            y = 2
            ```
            ",
        );
        let mf = super::parse("file.md", &source).unwrap();

        let [test1, test2] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected two tests");
        };

        assert_eq!(test1.name(), "file.md - One");
        assert_eq!(test2.name(), "file.md - Two");

        let [file] = test1.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.path, "test.py");
        assert_eq!(file.lang, "py");
        assert_eq!(file.code, "x = 1");

        let [file] = test2.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.path, "test.py");
        assert_eq!(file.lang, "py");
        assert_eq!(file.code, "y = 2");
    }

    #[test]
    fn multiple_file_tests() {
        let source = dedent(
            "
            # One

            ```py path=main.py
            from foo import y
            ```

            ```py path=foo.py
            y = 2
            ```

            # Two

            ```py
            y = 2
            ```
            ",
        );
        let mf = super::parse("file.md", &source).unwrap();

        let [test1, test2] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected two tests");
        };

        assert_eq!(test1.name(), "file.md - One");
        assert_eq!(test2.name(), "file.md - Two");

        let [main, foo] = test1.files().collect::<Vec<_>>()[..] else {
            panic!("expected two files");
        };

        assert_eq!(main.path, "main.py");
        assert_eq!(main.lang, "py");
        assert_eq!(main.code, "from foo import y");

        assert_eq!(foo.path, "foo.py");
        assert_eq!(foo.lang, "py");
        assert_eq!(foo.code, "y = 2");

        let [file] = test2.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.path, "test.py");
        assert_eq!(file.lang, "py");
        assert_eq!(file.code, "y = 2");
    }

    #[test]
    fn custom_file_path() {
        let source = dedent(
            "
            ```py path=foo.py
            x = 1
            ```
            ",
        );
        let mf = super::parse("file.md", &source).unwrap();

        let [test] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected one test");
        };
        let [file] = test.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.path, "foo.py");
        assert_eq!(file.lang, "py");
        assert_eq!(file.code, "x = 1");
    }

    #[test]
    fn multi_line_file() {
        let source = dedent(
            "
            ```py
            x = 1
            y = 2
            ```
            ",
        );
        let mf = super::parse("file.md", &source).unwrap();

        let [test] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected one test");
        };
        let [file] = test.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.code, "x = 1\ny = 2");
    }

    #[test]
    fn empty_file() {
        let source = dedent(
            "
            ```py
            ```
            ",
        );

        let mf = super::parse("file.md", &source).unwrap();

        let [test] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected one test");
        };
        let [file] = test.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.code, "");
    }

    #[test]
    fn no_lang() {
        let source = dedent(
            "
            ```
            x = 10
            ```
            ",
        );

        let mf = super::parse("file.md", &source).unwrap();

        let [test] = &mf.tests().collect::<Vec<_>>()[..] else {
            panic!("expected one test");
        };
        let [file] = test.files().collect::<Vec<_>>()[..] else {
            panic!("expected one file");
        };

        assert_eq!(file.code, "x = 10");
    }

    #[test]
    fn no_header_inside_test() {
        let source = dedent(
            "
            # One

            ```py
            x = 1
            ```

            ## Two
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(
            err.to_string(),
            "Header 'Two' not valid inside a test case; parent 'One' has code files."
        );
    }

    #[test]
    fn invalid_config_item_no_equals() {
        let source = dedent(
            "
            ```py foo
            x = 1
            ```
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(err.to_string(), "Invalid config item `foo`.");
    }

    #[test]
    fn invalid_config_item_too_many_equals() {
        let source = dedent(
            "
            ```py foo=bar=baz
            x = 1
            ```
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(err.to_string(), "Invalid config item `foo=bar=baz`.");
    }

    #[test]
    fn invalid_config_item_duplicate() {
        let source = dedent(
            "
            ```py foo=bar foo=baz
            x = 1
            ```
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(err.to_string(), "Duplicate config item `foo=baz`.");
    }

    #[test]
    fn no_duplicate_name_files_in_test() {
        let source = dedent(
            "
            ```py
            x = 1
            ```

            ```py
            y = 2
            ```
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(
            err.to_string(),
            "Test `file.md` has duplicate files named `test.py`. \
            (This is the default filename; consider giving some files an explicit name \
            with `path=...`.)"
        );
    }

    #[test]
    fn no_duplicate_name_files_in_test_non_default() {
        let source = dedent(
            "
            ```py path=foo.py
            x = 1
            ```

            ```py path=foo.py
            y = 2
            ```
            ",
        );
        let err = super::parse("file.md", &source).expect_err("Should fail to parse");
        assert_eq!(
            err.to_string(),
            "Test `file.md` has duplicate files named `foo.py`."
        );
    }
}
