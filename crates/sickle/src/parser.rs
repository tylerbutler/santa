//! Building the mutable syntax tree from CCL source.
//!
//! The tree is the single source of truth: every node keeps the exact source
//! text it came from, so an unmodified document renders back byte-for-byte,
//! while nodes created or edited afterwards render in canonical form.

use crate::error::ParseError;
use crate::item::Item;
use crate::lexer::{self, LexedEntry};
use crate::options::Options;
use crate::repr::{Decor, Key};
use crate::table::{Array, Table};
use crate::value::Value;

/// Maximum nesting depth, guarding against stack exhaustion on adversarial
/// input.
pub(crate) const MAX_DEPTH: usize = 64;

/// Parse `input` into a root [`Table`], plus the document's trailing trivia.
pub(crate) fn parse(input: &str, options: &Options) -> Result<Table, ParseError> {
    let processed = options.process_crlf(input);
    let (lines, trailing_newline) = lexer::physical_lines(&processed);
    let offsets = line_offsets(&lines);

    let mut ctx = Context {
        source: &processed,
        options,
    };
    ctx.build_block(&lines, &offsets, 0, "", trailing_newline, 0)
}

/// Byte offset of the start of each physical line.
fn line_offsets(lines: &[&str]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(lines.len());
    let mut cursor = 0usize;
    for line in lines {
        offsets.push(cursor);
        cursor += line.len() + 1;
    }
    offsets
}

struct Context<'a> {
    source: &'a str,
    options: &'a Options,
}

impl Context<'_> {
    /// Build one block of entries.
    ///
    /// `lead` is the raw text that separates the block from whatever preceded
    /// it: empty for the document root and for a block whose first entry starts
    /// on the parent's header line, otherwise the remainder of that header line
    /// plus its newline. `trailing_newline` records whether the block's last
    /// line ends with a newline; only the document root ever sets it.
    #[allow(clippy::too_many_arguments)]
    fn build_block(
        &mut self,
        lines: &[&str],
        offsets: &[usize],
        block_offset: usize,
        lead: &str,
        trailing_newline: bool,
        depth: usize,
    ) -> Result<Table, ParseError> {
        if depth > MAX_DEPTH {
            return Err(ParseError::at_offset(
                format!("maximum nesting depth ({MAX_DEPTH}) exceeded"),
                self.source,
                block_offset,
            ));
        }

        let entries = lexer::lex(lines, self.options);
        let mut table = Table::new();
        let mut cursor = 0usize;
        let mut pending = String::new();
        let mut first = true;

        for entry in &entries {
            for line in &lines[cursor..entry.start] {
                pending.push_str(line);
                pending.push('\n');
            }
            cursor = entry.end + 1;

            if is_comment(entry, lines) {
                for line in &lines[entry.start..=entry.end] {
                    pending.push_str(line);
                    pending.push('\n');
                }
                continue;
            }

            let mut prefix = String::new();
            if first {
                prefix.push_str(lead);
            } else {
                prefix.push('\n');
            }
            prefix.push_str(&pending);
            pending.clear();
            first = false;

            let header = lines[entry.header_end];
            let indent_len = header.len() - header.trim_start().len();
            let start_indent = {
                let line = lines[entry.start];
                &line[..line.len() - line.trim_start().len()]
            };
            prefix.push_str(start_indent);

            let (key_repr, key_gap, value_region, value_offset) =
                split_header(lines, offsets, entry, indent_len);

            let mut suffix = key_gap;
            if entry.delimiter.is_some() {
                suffix.push('=');
            }

            let key = Key::new(entry.key.clone())
                .with_repr(key_repr)
                .with_decor(Decor::new(prefix, suffix));

            let item =
                self.build_item(lines, offsets, entry, &value_region, value_offset, depth)?;
            table.push_entry(key, item);
        }

        for line in &lines[cursor..] {
            pending.push_str(line);
            pending.push('\n');
        }

        let mut trailing = String::new();
        if pending.is_empty() {
            if trailing_newline {
                trailing.push('\n');
            }
        } else {
            if first {
                trailing.push_str(lead);
            } else {
                trailing.push('\n');
            }
            if trailing_newline {
                trailing.push_str(&pending);
            } else {
                trailing.push_str(pending.strip_suffix('\n').unwrap_or(&pending));
            }
        }
        table.set_trailing(trailing);

        Ok(table)
    }

    /// Decide whether an entry's value is a scalar, a nested block, or a list.
    fn build_item(
        &mut self,
        lines: &[&str],
        offsets: &[usize],
        entry: &LexedEntry,
        value_region: &str,
        value_offset: usize,
        depth: usize,
    ) -> Result<Item, ParseError> {
        let scalar = || {
            Item::Value(Value::from_parts(
                entry.value.clone(),
                value_region.into(),
                Decor::default(),
            ))
        };

        // Only multi-line values that contain a delimiter can be nested CCL.
        if entry.value.contains('\n') && entry.value.contains('=') {
            let inline = inline_text(lines, entry);
            let has_inline = !inline.trim().is_empty();

            let (child_lines, child_offsets, lead) = if has_inline {
                let mut child_lines: Vec<&str> = vec![inline];
                let mut child_offsets = vec![value_offset];
                child_lines.extend_from_slice(&lines[entry.header_end + 1..=entry.end]);
                child_offsets.extend_from_slice(&offsets[entry.header_end + 1..=entry.end]);
                (child_lines, child_offsets, String::new())
            } else {
                if entry.end <= entry.header_end {
                    return Ok(scalar());
                }
                // Whatever trailed the `=` on the header line (a stray carriage
                // return, or trailing spaces) belongs to the block's opening.
                let lead = format!("{inline}\n");
                (
                    lines[entry.header_end + 1..=entry.end].to_vec(),
                    offsets[entry.header_end + 1..=entry.end].to_vec(),
                    lead,
                )
            };

            let child_offset = child_offsets.first().copied().unwrap_or(value_offset);
            let nested = self.build_block(
                &child_lines,
                &child_offsets,
                child_offset,
                &lead,
                false,
                depth + 1,
            )?;

            if !nested.is_empty() && nested.keys().all(is_valid_key) {
                return Ok(finish_block(nested));
            }
            return Ok(scalar());
        }

        // A single-line value that begins with whitespace is an indented child
        // line rather than value text (only reachable when tabs are preserved).
        if entry.value.starts_with(' ') || entry.value.starts_with('\t') {
            let trimmed = entry.value.trim();
            if !trimmed.is_empty() && !trimmed.contains('=') {
                let gap_len = value_region.len() - value_region.trim_start().len();
                let mut child = Table::new();
                child.push_entry(
                    Key::new(trimmed)
                        .with_repr(value_region.trim_start())
                        .with_decor(Decor::new(&value_region[..gap_len], "")),
                    Item::Value(Value::from_parts(
                        String::new(),
                        "".into(),
                        Decor::default(),
                    )),
                );
                return Ok(Item::Table(child));
            }
        }

        Ok(scalar())
    }
}

/// The raw text following `=` on the header line.
fn inline_text<'a>(lines: &[&'a str], entry: &LexedEntry) -> &'a str {
    match entry.delimiter {
        Some(delimiter) => &lines[entry.header_end][delimiter + 1..],
        None => "",
    }
}

/// Split an entry's header into key text, the gap before `=`, and the raw value
/// region (everything after `=`, including continuation lines).
fn split_header(
    lines: &[&str],
    offsets: &[usize],
    entry: &LexedEntry,
    indent_len: usize,
) -> (String, String, String, usize) {
    let header = lines[entry.header_end];

    let (key_raw, value_region, value_offset) = match entry.delimiter {
        Some(delimiter) => {
            let key_raw = if entry.start == entry.header_end {
                header[indent_len..delimiter].to_string()
            } else {
                let mut raw = lines[entry.start][key_indent(lines[entry.start])..].to_string();
                for line in &lines[entry.start + 1..entry.header_end] {
                    raw.push('\n');
                    raw.push_str(line);
                }
                raw.push('\n');
                raw.push_str(&header[..delimiter]);
                raw
            };
            let mut region = header[delimiter + 1..].to_string();
            for line in &lines[entry.header_end + 1..=entry.end] {
                region.push('\n');
                region.push_str(line);
            }
            let offset = offsets[entry.header_end] + delimiter + 1;
            (key_raw, region, offset)
        }
        None => {
            // No delimiter on this line: the key is the whole line and any
            // continuation lines form the value region.
            let key_raw = header[indent_len..].to_string();
            let mut region = String::new();
            for line in &lines[entry.header_end + 1..=entry.end] {
                region.push('\n');
                region.push_str(line);
            }
            let offset = offsets[entry.header_end] + header.len();
            (key_raw, region, offset)
        }
    };

    let trimmed_key = key_raw.trim_end_matches([' ', '\t']);
    let gap = key_raw[trimmed_key.len()..].to_string();
    (trimmed_key.to_string(), gap, value_region, value_offset)
}

fn key_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Normalize a parsed block: a block whose entries are all bare list items
/// (`= value`) becomes a list.
fn finish_block(table: Table) -> Item {
    if !table.is_bare_list() {
        return Item::Table(table);
    }
    let mut array = Array::new();
    let trailing = table.trailing().clone();
    for entry in table.entries() {
        let decor = Decor::new(
            entry.key.decor().prefix().clone(),
            entry.key.decor().suffix().clone(),
        );
        array.push_element(decor, entry.item.clone());
    }
    array.set_trailing(trailing);
    Item::Array(array)
}

/// Whether a lexed entry is a comment line (`/= text`).
///
/// Comments are trivia in the syntax tree, not table keys, so they are attached
/// to the following entry's decoration.
fn is_comment(entry: &LexedEntry, lines: &[&str]) -> bool {
    entry.key == "/" && entry.start == entry.end && lines[entry.start].trim_start().starts_with('/')
}

/// Whether a recursively parsed key looks like real CCL rather than a
/// misinterpreted value string.
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return true;
    }
    if key.starts_with('-') {
        return false;
    }
    if key.contains(" = ") || key.contains(" =\t") {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::render_table;

    fn round_trip(input: &str) -> String {
        let table = parse(input, &Options::new()).unwrap();
        let mut out = String::new();
        render_table(&table, 0, &mut out);
        out
    }

    #[test]
    fn simple_entries_round_trip() {
        for input in [
            "a = 1\n",
            "a = 1",
            "a = 1\nb = 2\n",
            "  a = 1\n  b = 2\n",
            "a=1\n",
            "a  =  1\n",
        ] {
            assert_eq!(round_trip(input), input, "input: {input:?}");
        }
    }

    #[test]
    fn nested_blocks_round_trip() {
        let input = "server =\n  host = localhost\n  port = 8080\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        let server = table.get("server").unwrap().as_table().unwrap();
        assert_eq!(server.get("host").unwrap().as_str(), Some("localhost"));
    }

    #[test]
    fn bare_lists_become_arrays() {
        let input = "servers =\n  = web1\n  = web2\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        let array = table.get("servers").unwrap().as_array().unwrap();
        assert_eq!(array.as_strings(), Some(vec!["web1", "web2"]));
    }

    #[test]
    fn comments_and_blank_lines_are_trivia() {
        let input = "/= header\n\nname = app\n\n/= trailing\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(table.get("name").unwrap().as_str(), Some("app"));
    }

    #[test]
    fn duplicate_keys_stay_ordered() {
        let input = "item = a\nother = x\nitem = b\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        assert_eq!(table.count("item"), 2);
    }

    #[test]
    fn multiline_scalars_keep_their_text() {
        let input = "script =\n  #!/bin/sh\n  echo hi\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        assert_eq!(
            table.get("script").unwrap().as_str(),
            Some("\n  #!/bin/sh\n  echo hi")
        );
    }

    #[test]
    fn deep_nesting_is_bounded() {
        let mut input = String::from("a = b");
        for _ in 0..=MAX_DEPTH {
            let indented: String = input.lines().map(|l| format!("  {l}\n")).collect();
            input = format!("outer =\n{indented}");
        }
        let err = parse(&input, &Options::new()).unwrap_err();
        assert!(err.message().contains("nesting depth"));
    }

    #[test]
    fn interleaved_bare_items_stay_a_table() {
        let input = "name = Alice\n= first\ncfg =\n  port = 1\n= second\n";
        assert_eq!(round_trip(input), input);
        let table = parse(input, &Options::new()).unwrap();
        assert_eq!(table.count(""), 2);
        assert!(!table.is_bare_list());
    }
}
