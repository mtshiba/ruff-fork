use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::pyupgrade::helpers::curly_escape;
use once_cell::sync::Lazy;
use regex::Regex;
use rustpython_ast::{Expr, ExprKind};

// Tests: https://github.com/asottile/pyupgrade/blob/main/tests/features/percent_format_test.py
// Code: https://github.com/asottile/pyupgrade/blob/97ed6fb3cf2e650d4f762ba231c3f04c41797710/pyupgrade/_plugins/percent_format.py#L48
// TODO: do not forget--keep-percent-format as a way to ignore this rule

static MAPPING_KEY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\(([^()]*)\)").unwrap());
static CONVERSION_FLAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[#0+ -]*").unwrap());
static WIDTH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\*|\d*)").unwrap());
static PRECISION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\.(?:\*|\d*))?").unwrap());
static LENGTH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[hlL]?").unwrap());

#[derive(Debug, PartialEq, Clone)]
struct PercentFormatPart {
    key: Option<String>,
    conversion_flag: Option<String>,
    width: Option<String>,
    precision: Option<String>,
    conversion: String,
}

impl PercentFormatPart {
    fn new(
        key: Option<String>,
        conversion_flag: Option<String>,
        width: Option<String>,
        precision: Option<String>,
        conversion: String,
    ) -> Self {
        Self {
            key,
            conversion_flag,
            width,
            precision,
            conversion,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct PercentFormat {
    item: String,
    parts: Option<PercentFormatPart>,
}

impl PercentFormat {
    fn new(item: String, parts: Option<PercentFormatPart>) -> Self {
        Self { item, parts }
    }
}

/// Gets the match from a regex and potentiall updated the value of a given integer
fn get_flag<'a>(regex: &'a Lazy<Regex>, string: &'a str, position: &mut usize) -> Option<String> {
    let flag_match = regex.find_at(string, *position);
    if let Some(flag_match) = flag_match {
        *position = flag_match.end();
        let the_string = flag_match.as_str().to_string();
        if the_string.is_empty() {
            None
        } else {
            Some(the_string)
        }
    } else {
        None
    }
}

fn parse_percent_format(string: &str) -> Vec<PercentFormat> {
    let mut string_start = 0;
    let mut string_end = 0;
    let mut in_fmt = false;
    let mut formats: Vec<PercentFormat> = vec![];

    let mut i = 0;
    while i < string.len() {
        if !in_fmt {
            i = match string[i..].find('%') {
                None => {
                    let fmt_full = PercentFormat::new(string[string_start..].to_string(), None);
                    formats.push(fmt_full);
                    return formats;
                }
                // Since we cut off the part of the string before `i` in the beginning, we need to
                // add it back to get the proper index
                Some(item) => item + i,
            };
            string_end = i;
            i += 1;
            in_fmt = true;
        } else {
            let mut key: Option<String> = None;
            if let Some(key_item) = MAPPING_KEY_RE.captures(&string[i..]) {
                if let Some(match_item) = key_item.get(1) {
                    key = Some(match_item.as_str().to_string());
                    // Have to use another regex because the rust Capture object does not have an
                    // end() method
                    i = MAPPING_KEY_RE.find_at(string, i).unwrap().end();
                }
            };

            let conversion_flag = get_flag(&CONVERSION_FLAG_RE, string, &mut i);
            let width = get_flag(&WIDTH_RE, string, &mut i);
            let precision = get_flag(&PRECISION_RE, string, &mut i);

            // length modifier is ignored
            i = LENGTH_RE.find_at(string, i).unwrap().end();
            // I use clone because nth consumes characters before position n
            let conversion = match string.clone().chars().nth(i) {
                None => panic!("end-of-string while parsing format"),
                Some(conv_item) => conv_item,
            };
            i += 1;

            let fmt = PercentFormatPart::new(
                key,
                conversion_flag,
                width,
                precision,
                conversion.to_string(),
            );
            let fmt_full =
                PercentFormat::new(string[string_start..string_end].to_string(), Some(fmt));
            formats.push(fmt_full);

            in_fmt = false;
            string_start = i;
        }
    }

    if in_fmt {
        panic!("end-of-string while parsing format");
    }
    formats
}

/// Removes the first instance of a given element from a vector
fn remove(vec: &mut Vec<char>, item: char) {
    let index = vec.iter().position(|&x| x == item).unwrap();
    vec.remove(index);
}

fn simplify_conversion_flag(flag: &str) -> String {
    let mut parts: Vec<char> = vec![];
    for mut character in flag.chars() {
        if parts.contains(&character) {
            continue;
        }
        if character == '-' {
            character = '<';
        }
        parts.push(character);
        if character == '<' && parts.contains(&'0') {
            remove(&mut parts, '0');
        } else if character == '+' && parts.contains(&' ') {
            remove(&mut parts, ' ');
        }
    }
    String::from_iter(parts)
}

/// Returns true if any of conversion_flag, width, precision, and conversion are a non-empty string
fn any_percent_format(pf: &PercentFormatPart) -> bool {
    if let Some(conversion_flag) = &pf.conversion_flag {
        if let Some(width) = &pf.width {
            if let Some(precision) = &pf.precision {
                return !conversion_flag.is_empty()
                    || !width.is_empty()
                    || !precision.is_empty()
                    || !pf.conversion.is_empty();
            }
        }
    }
    false
}

fn handle_part(part: &PercentFormat) -> String {
    let mut string = part.item.clone();
    string = curly_escape(&string);
    let mut fmt = match part.parts.clone() {
        None => return string,
        Some(item) => item,
    };

    if fmt.conversion == "%".to_string() {
        string.push('%');
        return string;
    }
    let mut parts = vec![string, "{".to_string()];
    if fmt.conversion == "s".to_string() {
        fmt.conversion = "".to_string();
    }
    if let Some(key_item) = &fmt.key {
        parts.push(key_item.to_string());
    }
    let converter: String;
    if fmt.conversion == "r".to_string() || fmt.conversion == "a".to_string() {
        converter = format!("!{}", fmt.conversion);
        fmt.conversion = "".to_string();
    } else {
        converter = "".to_string();
    }
    if any_percent_format(&fmt) {
        parts.push(":".to_string());
    }
    if let Some(conversion_flag) = &fmt.conversion_flag {
        if !conversion_flag.is_empty() {
            let simplified = simplify_conversion_flag(&conversion_flag);
            parts.push(simplified);
        }
    }
    if let Some(width) = &fmt.width {
        if !width.is_empty() {
            parts.push(width.to_string());
        }
    }

    if let Some(precision) = &fmt.precision {
        if !precision.is_empty() {
            parts.push(precision.to_string());
        }
    }
    if !fmt.conversion.is_empty() {
        parts.push(fmt.conversion.clone());
    }
    for character in converter.chars() {
        parts.push(character.to_string())
    }
    parts.push("}".to_string());
    String::from_iter(parts)
}

fn percent_to_format(string: &str) -> String {
    let mut final_string = String::new();
    for part in parse_percent_format(string) {
        final_string.push_str(&handle_part(&part));
    }
    final_string
}

fn fix_percent_format_tuple(checker: &mut Checker, left: &Expr, right: &Expr, left_string: &str) {
    // Pyupgrade explicitly checks for ' % (' before running, but I am not sure the value of this
    // (pyupgrade itself says it is overly timid). The one edge case I considered was a multi line
    // format statement, but worst-case scenario we go over the limit and black fixes it. Let me
    // know if you want this check implemented
    let right_range = Range::new(right.location, right.end_location.unwrap());
    let right_string = checker.locator.slice_source_code_range(&right_range);
    let mut cleaned_string = percent_to_format(left_string);
    cleaned_string.push_str(".format");
    cleaned_string.push_str(&right_string);
    println!("{}", cleaned_string);
}

fn fix_percent_format_dict(checker: &mut Checker, left: &Expr, right: &Expr) {}

/// Returns true if any of conversion_flag, width, and precision are a non-empty string
fn get_nontrivial_fmt(pf: &PercentFormatPart) -> bool {
    if let Some(conversion_flag) = &pf.conversion_flag {
        if let Some(width) = &pf.width {
            if let Some(precision) = &pf.precision {
                return !conversion_flag.is_empty() || !width.is_empty() || !precision.is_empty();
            }
        }
    }
    false
}

/// UP031
pub fn printf_string_formatting(checker: &mut Checker, left: &Expr, right: &Expr) {
    //This is just to get rid of the 10,000 lint errors
    let left_range = Range::new(left.location, left.end_location.unwrap());
    let left_string = checker.locator.slice_source_code_range(&left_range);
    let parsed = parse_percent_format(&left_string);
    // Rust does not have a for else statement so we have to do this
    let mut no_breaks = true;
    for item in parsed {
        let fmt = match item.parts {
            None => continue,
            Some(item) => item,
        };
        // timid: these require out-of-order parameter consumption
        if fmt.width == Some("*".to_string()) || fmt.precision == Some("*".to_string()) {
            no_breaks = false;
            break;
        }
        // these conversions require modification of parameters
        if vec!["d", "i", "u", "c"].contains(&&fmt.conversion[..]) {
            no_breaks = false;
            break;
        }
        // timid: py2: %#o formats different from {:#o} (--py3?)
        if fmt
            .conversion_flag
            .clone()
            .unwrap_or_default()
            .contains("#")
            && fmt.conversion == "o"
        {
            no_breaks = false;
            break;
        }
        // no equivalent in format
        if let Some(key) = &fmt.key {
            if key.is_empty() {
                no_breaks = false;
                break;
            }
        }
        // timid: py2: conversion is subject to modifiers (--py3?)
        let nontrivial_fmt = get_nontrivial_fmt(&fmt);
        if fmt.conversion == "%".to_string() && nontrivial_fmt {
            no_breaks = false;
            break;
        }
        // no equivalent in format
        if vec!["a", "r"].contains(&&fmt.conversion[..]) && nontrivial_fmt {
            no_breaks = false;
            break;
        }
        // %s with None and width is not supported
        if let Some(width) = &fmt.width {
            if !width.is_empty() && fmt.conversion == "s".to_string() {
                no_breaks = false;
                break;
            }
        }
        // all dict substitutions must be named
        if let ExprKind::Dict { .. } = &right.node {
            // Technically a value of "" would also count as `not key`, BUT we already have a check
            // above for this
            if fmt.key.is_none() {
                no_breaks = false;
                break;
            }
        }
    }
    if no_breaks {
        match &right.node {
            ExprKind::Tuple { .. } => {
                fix_percent_format_tuple(checker, left, right, &left_string);
            }
            ExprKind::Dict { .. } => {
                fix_percent_format_dict(checker, left, right);
            }
            _ => {}
        }
    }
}
//Pyupgrade has a bunch of tests specific to `parse_percent_format`, I figured it wouldn't hurt to
//add them
#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_parse_percent_format_none() {
        let sample = "\"\"";
        let e1 = PercentFormat::new("\"\"".to_string(), None);
        let expected = vec![e1];

        let received = parse_percent_format(sample);
        assert_eq!(received, expected);
    }

    #[test]
    fn test_parse_percent_format_double_two() {
        let sample = "\"%s two! %s\"";
        let sube1 = PercentFormatPart::new(None, None, None, None, "s".to_string());
        let e1 = PercentFormat::new(" two! ".to_string(), Some(sube1.clone()));
        let e2 = PercentFormat::new("\"".to_string(), Some(sube1));
        let e3 = PercentFormat::new("\"".to_string(), None);
        let expected = vec![e2, e1, e3];

        let received = parse_percent_format(sample);
        assert_eq!(received, expected);
    }

    #[test_case("\"%ld\"",PercentFormatPart::new(None, None, None, None, "d".to_string()); "two letter")]
    #[test_case( "\"%.*f\"",PercentFormatPart::new(None, None, None, Some(".*".to_string()), "f".to_string()); "dot star letter")]
    #[test_case( "\"%.5f\"",PercentFormatPart::new(None, None, None, Some(".5".to_string()), "f".to_string()); "dot number letter")]
    #[test_case( "\"%.f\"",PercentFormatPart::new(None, None, None, Some(".".to_string()), "f".to_string()); "dot letter")]
    #[test_case( "\"%*d\"",PercentFormatPart::new(None, None, Some("*".to_string()), None, "d".to_string()); "star d")]
    #[test_case( "\"%5d\"",PercentFormatPart::new(None, None, Some("5".to_string()), None, "d".to_string()); "number letter")]
    #[test_case( "\"% #0-+d\"",PercentFormatPart::new(None, Some(" #0-+".to_string()), None, None, "d".to_string()); "hastag and symbols")]
    #[test_case( "\"%#o\"",PercentFormatPart::new(None, Some("#".to_string()), None, None, "o".to_string()); "format hashtag")]
    #[test_case( "\"%()s\"",PercentFormatPart::new(Some("".to_string()), None, None, None, "s".to_string()); "empty paren")]
    #[test_case( "\"%(hi)s\"",PercentFormatPart::new(Some("hi".to_string()), None, None, None, "s".to_string()); "word in paren")]
    #[test_case( "\"%s\"",PercentFormatPart::new(None, None, None, None, "s".to_string()); "format s")]
    #[test_case( "\"%%\"",PercentFormatPart::new(None, None, None, None, "%".to_string()); "format double percentage")]
    fn test_parse_percent_format(sample: &str, expected: PercentFormatPart) {
        let e1 = PercentFormat::new("\"".to_string(), Some(expected));
        let e2 = PercentFormat::new("\"".to_string(), None);
        let expected = vec![e1, e2];

        let received = parse_percent_format(sample);
        assert_eq!(received, expected);
    }

    #[test]
    fn test_parse_percent_format_everything() {
        let sample = "\"%(complete)#4.4f\"";
        let sube1 = PercentFormatPart::new(
            Some("complete".to_string()),
            Some("#".to_string()),
            Some("4".to_string()),
            Some(".4".to_string()),
            "f".to_string(),
        );
        let e1 = PercentFormat::new("\"".to_string(), Some(sube1));
        let e2 = PercentFormat::new("\"".to_string(), None);
        let expected = vec![e1, e2];

        let received = parse_percent_format(sample);
        assert_eq!(received, expected);
    }

    #[test_case("%s", "{}"; "simple string")]
    #[test_case("%%s", "%{}"; "two percents")]
    #[test_case("%(foo)s", "{foo}"; "word in string")]
    #[test_case("%2f", "{:2f}"; "formatting in string")]
    #[test_case("%r", "{!r}"; "format an r")]
    #[test_case("%a", "{!a}"; "format an a")]
    fn test_percent_to_format(sample: &str, expected: &str) {
        let received = percent_to_format(sample);
        assert_eq!(received, expected);
    }

    #[test_case("", ""; "preserve blanks")]
    #[test_case(" ", " "; "preserve one space")]
    #[test_case("  ", " "; "two spaces to one")]
    #[test_case("#0- +", "#<+"; "complex format")]
    #[test_case("-", "<"; "simple format")]
    fn test_simplify_conversion_flag(sample: &str, expected: &str) {
        let received = simplify_conversion_flag(sample);
        assert_eq!(received, expected);
    }
}
