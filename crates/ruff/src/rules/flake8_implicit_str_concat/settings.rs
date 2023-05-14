//! Settings for the `flake8-implicit-str-concat` plugin.

use serde::{Deserialize, Serialize};

use ruff_macros::{CacheKey, ConfigurationOptions};

use crate::settings::configuration::CombinePluginOptions;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, ConfigurationOptions)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    rename = "Flake8ImplicitStrConcatOptions"
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Options {
    #[option(
        default = r#"true"#,
        value_type = "bool",
        example = r#"
            allow-multiline = false
        "#
    )]
    /// Whether to allow implicit string concatenations for multiline strings.
    /// By default, implicit concatenations of multiline strings are
    /// allowed (but continuation lines, delimited with a backslash, are
    /// prohibited).
    ///
    /// Note that setting `allow-multiline = false` should typically be coupled
    /// with disabling `explicit-string-concatenation` (`ISC003`). Otherwise,
    /// both explicit and implicit multiline string concatenations will be seen
    /// as violations.
    pub allow_multiline: Option<bool>,
}

#[derive(Debug, CacheKey)]
pub struct Settings {
    pub allow_multiline: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            allow_multiline: true,
        }
    }
}

impl From<Options> for Settings {
    fn from(options: Options) -> Self {
        Self {
            allow_multiline: options.allow_multiline.unwrap_or(true),
        }
    }
}

impl From<Settings> for Options {
    fn from(settings: Settings) -> Self {
        Self {
            allow_multiline: Some(settings.allow_multiline),
        }
    }
}

impl CombinePluginOptions for Options {
    fn combine(self, other: Self) -> Self {
        Self {
            allow_multiline: self.allow_multiline.or(other.allow_multiline),
        }
    }
}
