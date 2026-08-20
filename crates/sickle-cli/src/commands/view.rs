use anyhow::Result;
use colored::Colorize;
use sickle::{DocumentMut, Item, Table};
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub(crate) struct ViewArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,
}

pub(crate) fn run(args: ViewArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    let document = DocumentMut::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    print_table(document.as_table(), 0);
    Ok(())
}

fn print_table(table: &Table, indent: usize) {
    let pad = " ".repeat(indent);
    for (key, item) in table.iter_keys() {
        // Comments and blank lines live in the key's decoration.
        print_comments(key.decor().prefix().or(""), &pad);

        // An empty key is a bare list item, written `= value`.
        let label = (!key.get().is_empty()).then(|| key.get().yellow().to_string());
        print_entry(label, item, indent, &pad);
    }
    print_comments(table.trailing().or(""), &pad);
}

fn print_entry(label: Option<String>, item: &Item, indent: usize, pad: &str) {
    let equals = "=".dimmed();
    let head = match &label {
        Some(key) => format!("{pad}{key} {equals}"),
        None => format!("{pad}{equals}"),
    };

    match item {
        Item::None => println!("{head}"),
        Item::Value(value) if value.as_str().is_empty() => println!("{head}"),
        Item::Value(value) => println!("{head} {}", value.as_str().cyan()),
        Item::Table(nested) => {
            println!("{head}");
            print_table(nested, indent + 2);
        }
        Item::Array(array) => {
            println!("{head}");
            let child_pad = " ".repeat(indent + 2);
            for element in array.iter() {
                print_entry(None, element, indent + 2, &child_pad);
            }
        }
    }
}

fn print_comments(raw: &str, pad: &str) {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/=") {
            println!("{pad}{}", trimmed.dimmed());
        }
    }
}
