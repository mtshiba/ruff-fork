//! Generate typescript file defining options to be used by the web playground.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use itertools::Itertools;
use ruff::settings::options::Options;
use ruff::settings::options_base::{ConfigurationOptions, OptionEntry, OptionField};

#[derive(Args)]
pub struct Cli {
    /// Write the generated table to stdout (rather than to `TODO`).
    #[arg(long)]
    pub(crate) dry_run: bool,
}

fn emit_field(output: &mut String, field: &OptionField) {
    output.push_str(&textwrap::indent(
        &textwrap::dedent(&format!(
            "
        {{
            \"name\": \"{}\",
            \"default\": '{}',
            \"type\": '{}',
        }},",
            field.name, field.default, field.value_type
        )),
        "    ",
    ));
}

pub fn main(cli: &Cli) -> Result<()> {
    let mut output = String::new();

    // Generate all the top-level fields.
    output.push_str(&format!("{{\"name\": \"{}\", \"fields\": [", "globals"));
    for field in Options::get_available_options()
        .into_iter()
        .filter_map(|entry| {
            if let OptionEntry::Field(field) = entry {
                Some(field)
            } else {
                None
            }
        })
        // Filter out options that don't make sense in the playground.
        .filter(|field| {
            !matches!(
                field.name,
                "src"
                    | "fix"
                    | "format"
                    | "exclude"
                    | "extend"
                    | "extend-exclude"
                    | "fixable"
                    | "force-exclude"
                    | "ignore-init-module-imports"
                    | "respect-gitignore"
                    | "show-source"
                    | "cache-dir"
                    | "per-file-ignores"
            )
        })
        .sorted_by_key(|field| field.name)
    {
        emit_field(&mut output, &field);
    }
    output.push_str("\n]},\n");

    // Generate all the sub-groups.
    for group in Options::get_available_options()
        .into_iter()
        .filter_map(|entry| {
            if let OptionEntry::Group(group) = entry {
                Some(group)
            } else {
                None
            }
        })
        .sorted_by_key(|group| group.name)
    {
        output.push_str(&format!("{{\"name\": \"{}\", \"fields\": [", group.name));
        for field in group
            .fields
            .iter()
            .filter_map(|entry| {
                if let OptionEntry::Field(field) = entry {
                    Some(field)
                } else {
                    None
                }
            })
            .sorted_by_key(|field| field.name)
        {
            emit_field(&mut output, field);
        }
        output.push_str("\n]},\n");
    }

    let prefix = textwrap::dedent(
        r"
        // This file is auto-generated by `cargo dev generate-playground-options`.
        export interface OptionGroup {
            name: string;
            fields: {
                name: string;
                default: string;
                type: string;
            }[];
        };

        export const AVAILABLE_OPTIONS: OptionGroup[] = [
    ",
    );
    let postfix = "];";

    if cli.dry_run {
        print!("{output}");
    } else {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Failed to find root directory")
            .join("playground")
            .join("src")
            .join("ruff_options.ts");

        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file)?;
        write!(f, "{prefix}")?;
        write!(f, "{}", textwrap::indent(&output, "    "))?;
        write!(f, "{postfix}")?;
    }

    Ok(())
}
