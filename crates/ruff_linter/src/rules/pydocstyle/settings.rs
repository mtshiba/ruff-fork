//! Settings for the `pydocstyle` plugin.

use std::collections::BTreeSet;
use std::fmt;
use std::iter::FusedIterator;

use serde::{Deserialize, Serialize};

use ruff_macros::CacheKey;
use ruff_python_ast::name::QualifiedName;

use crate::display_settings;
use crate::registry::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CacheKey)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Convention {
    /// Use Google-style docstrings.
    Google,
    /// Use NumPy-style docstrings.
    Numpy,
    /// Use Sphinx-style docstrings.
    Sphinx,
    /// Use PEP257-style docstrings.
    Pep257,
}

impl Convention {
    pub const fn rules_to_be_ignored(self) -> &'static [Rule] {
        match self {
            Convention::Google => &[
                Rule::BlankLineAfterLastSection,
                Rule::DashedUnderlineAfterSection,
                Rule::DocstringStartsWithThis,
                Rule::EndsInPeriod,
                Rule::MultiLineSummarySecondLine,
                Rule::NewLineAfterSectionName,
                Rule::NonImperativeMood,
                Rule::OneBlankLineAfterClass,
                Rule::OneBlankLineBeforeClass,
                Rule::SectionUnderlineAfterName,
                Rule::SectionUnderlineMatchesSectionLength,
                Rule::SectionUnderlineNotOverIndented,
            ],
            Convention::Numpy => &[
                Rule::BlankLineAfterLastSection,
                Rule::EndsInPunctuation,
                Rule::MultiLineSummaryFirstLine,
                Rule::MultiLineSummarySecondLine,
                Rule::NoSignature,
                Rule::OneBlankLineBeforeClass,
                Rule::SectionNameEndsInColon,
                Rule::UndocumentedParam,
                Rule::UndocumentedPublicInit,
            ],
            Convention::Sphinx => &[
                Rule::BlankLineAfterLastSection,
                Rule::CapitalizeSectionName,
                Rule::DashedUnderlineAfterSection,
                Rule::DocstringStartsWithThis,
                Rule::EndsInPunctuation,
                Rule::MultiLineSummaryFirstLine,
                Rule::MultiLineSummarySecondLine,
                Rule::NewLineAfterSectionName,
                Rule::NoBlankLineAfterSection,
                Rule::NoBlankLineBeforeSection,
                Rule::OneBlankLineBeforeClass,
                Rule::SectionNameEndsInColon,
                Rule::SectionUnderlineAfterName,
                Rule::SectionUnderlineMatchesSectionLength,
                Rule::SectionUnderlineNotOverIndented,
                Rule::UndocumentedParam,
            ],
            Convention::Pep257 => &[
                Rule::BlankLineAfterLastSection,
                Rule::CapitalizeSectionName,
                Rule::DashedUnderlineAfterSection,
                Rule::DocstringStartsWithThis,
                Rule::EndsInPunctuation,
                Rule::MultiLineSummaryFirstLine,
                Rule::MultiLineSummarySecondLine,
                Rule::NewLineAfterSectionName,
                Rule::NoBlankLineAfterSection,
                Rule::NoBlankLineBeforeSection,
                Rule::OneBlankLineBeforeClass,
                Rule::SectionNameEndsInColon,
                Rule::SectionNotOverIndented,
                Rule::SectionUnderlineAfterName,
                Rule::SectionUnderlineMatchesSectionLength,
                Rule::SectionUnderlineNotOverIndented,
                Rule::UndocumentedParam,
            ],
        }
    }
}

impl fmt::Display for Convention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Google => write!(f, "google"),
            Self::Numpy => write!(f, "numpy"),
            Self::Sphinx => write!(f, "sphinx"),
            Self::Pep257 => write!(f, "pep257"),
        }
    }
}

#[derive(Debug, Clone, Default, CacheKey)]
pub struct Settings {
    convention: Option<Convention>,
    ignore_decorators: BTreeSet<String>,
    property_decorators: BTreeSet<String>,
}

impl Settings {
    #[must_use]
    pub fn new(
        convention: Option<Convention>,
        ignore_decorators: impl IntoIterator<Item = String>,
        property_decorators: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            convention,
            ignore_decorators: ignore_decorators.into_iter().collect(),
            property_decorators: property_decorators.into_iter().collect(),
        }
    }

    pub fn convention(&self) -> Option<Convention> {
        self.convention
    }

    pub fn ignore_decorators(&self) -> DecoratorIterator {
        DecoratorIterator::new(&self.ignore_decorators)
    }

    pub fn property_decorators(&self) -> DecoratorIterator {
        DecoratorIterator::new(&self.property_decorators)
    }
}

impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_settings! {
            formatter = f,
            namespace = "linter.pydocstyle",
            fields = [
                self.convention | optional,
                self.ignore_decorators | set,
                self.property_decorators | set
            ]
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DecoratorIterator<'a> {
    decorators: std::collections::btree_set::Iter<'a, String>,
}

impl<'a> DecoratorIterator<'a> {
    fn new(decorators: &'a BTreeSet<String>) -> Self {
        Self {
            decorators: decorators.iter(),
        }
    }
}

impl<'a> Iterator for DecoratorIterator<'a> {
    type Item = QualifiedName<'a>;

    fn next(&mut self) -> Option<QualifiedName<'a>> {
        self.decorators
            .next()
            .map(|deco| QualifiedName::from_dotted_name(deco))
    }
}

impl FusedIterator for DecoratorIterator<'_> {}

impl ExactSizeIterator for DecoratorIterator<'_> {
    fn len(&self) -> usize {
        self.decorators.len()
    }
}
