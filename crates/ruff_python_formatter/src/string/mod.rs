pub(crate) use normalize::{normalize_string, NormalizedString, StringNormalizer};
use ruff_python_ast::str::Quote;
use ruff_python_ast::StringLikePart;
use ruff_python_ast::{
    self as ast,
    str_prefix::{AnyStringPrefix, StringLiteralPrefix},
    AnyStringFlags, StringFlags,
};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::expression::expr_f_string::f_string_quoting;
use crate::prelude::*;
use crate::preview::is_f_string_formatting_enabled;
use crate::QuoteStyle;

pub(crate) mod docstring;
pub(crate) mod implicit;
mod normalize;

#[derive(Copy, Clone, Debug, Default)]
pub(crate) enum Quoting {
    #[default]
    CanChange,
    Preserve,
}

impl Format<PyFormatContext<'_>> for AnyStringPrefix {
    fn fmt(&self, f: &mut PyFormatter) -> FormatResult<()> {
        // Remove the unicode prefix `u` if any because it is meaningless in Python 3+.
        if !matches!(
            self,
            AnyStringPrefix::Regular(StringLiteralPrefix::Empty | StringLiteralPrefix::Unicode)
        ) {
            token(self.as_str()).fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct StringQuotes {
    triple: bool,
    quote_char: Quote,
}

impl Format<PyFormatContext<'_>> for StringQuotes {
    fn fmt(&self, f: &mut PyFormatter) -> FormatResult<()> {
        let quotes = match (self.quote_char, self.triple) {
            (Quote::Single, false) => "'",
            (Quote::Single, true) => "'''",
            (Quote::Double, false) => "\"",
            (Quote::Double, true) => "\"\"\"",
        };

        token(quotes).fmt(f)
    }
}

impl From<AnyStringFlags> for StringQuotes {
    fn from(value: AnyStringFlags) -> Self {
        Self {
            triple: value.is_triple_quoted(),
            quote_char: value.quote_style(),
        }
    }
}

impl TryFrom<QuoteStyle> for Quote {
    type Error = ();

    fn try_from(style: QuoteStyle) -> Result<Quote, ()> {
        match style {
            QuoteStyle::Single => Ok(Quote::Single),
            QuoteStyle::Double => Ok(Quote::Double),
            QuoteStyle::Preserve => Err(()),
        }
    }
}

impl From<Quote> for QuoteStyle {
    fn from(value: Quote) -> Self {
        match value {
            Quote::Single => QuoteStyle::Single,
            Quote::Double => QuoteStyle::Double,
        }
    }
}

// Extension trait that adds formatter specific helper methods to `StringLike`.
pub(crate) trait StringLikeExtensions {
    fn quoting(&self, source: &str) -> Quoting;

    fn is_multiline(&self, context: &PyFormatContext) -> bool;
}

impl StringLikeExtensions for ast::StringLike<'_> {
    fn quoting(&self, source: &str) -> Quoting {
        match self {
            Self::String(_) | Self::Bytes(_) => Quoting::CanChange,
            Self::FString(f_string) => f_string_quoting(f_string, source),
        }
    }

    fn is_multiline(&self, context: &PyFormatContext) -> bool {
        self.parts().any(|part| match part {
            StringLikePart::String(_) | StringLikePart::Bytes(_) => {
                part.flags().is_triple_quoted()
                    && context.source().contains_line_break(part.range())
            }
            StringLikePart::FString(f_string) => {
                fn contains_line_break_or_comments(
                    elements: &ast::FStringElements,
                    context: &PyFormatContext,
                ) -> bool {
                    elements.iter().any(|element| match element {
                        ast::FStringElement::Literal(literal) => {
                            context.source().contains_line_break(literal.range())
                        }
                        ast::FStringElement::Expression(expression) => {
                            if is_f_string_formatting_enabled(context) {
                                // Expressions containing comments can't be joined.
                                //
                                // Format specifiers needs to be checked as well. For example, the
                                // following should be considered multiline because the literal
                                // part of the format specifier contains a newline at the end
                                // (`.3f\n`):
                                //
                                // ```py
                                // x = f"hello {a + b + c + d:.3f
                                // } world"
                                // ```
                                context.comments().contains_comments(expression.into())
                                    || expression.format_spec.as_deref().is_some_and(|spec| {
                                        contains_line_break_or_comments(&spec.elements, context)
                                    })
                            } else {
                                // Multiline f-string expressions can't be joined if the f-string
                                // formatting is disabled because the string gets inserted in
                                // verbatim preserving the newlines.
                                //
                                // We don't need to check format specifiers here because the
                                // expression range already includes them.
                                context.source().contains_line_break(expression.range())
                            }
                        }
                    })
                }

                contains_line_break_or_comments(&f_string.elements, context)
            }
        })
    }
}
