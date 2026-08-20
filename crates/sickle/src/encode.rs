//! Rendering the syntax tree back to CCL text.
//!
//! Nodes that came from parsing carry their verbatim source text and are
//! emitted unchanged; nodes created or edited programmatically are emitted in
//! canonical form: two-space indentation, `key = value`, `key =` followed by an
//! indented block for nested data, and `= item` lines for lists.

use crate::item::Item;
use crate::table::{Array, ArrayKind, Table};
use crate::value::Value;

/// Indentation added for each nesting level in canonical output.
pub(crate) const INDENT_STEP: usize = 2;

/// Render `table`'s entries, indenting canonical output by `indent` spaces.
pub(crate) fn render_table(table: &Table, indent: usize, out: &mut String) {
    for entry in table.entries() {
        if entry.item.is_none() {
            continue;
        }

        if let Item::Array(array) = &entry.item {
            if array.kind() == ArrayKind::RepeatedKey {
                render_repeated_key(entry, array, indent, out);
                continue;
            }
        }

        push_prefix(entry.key.decor().prefix().as_str(), indent, out);
        out.push_str(entry.key.repr().or(entry.key.get()));
        out.push_str(entry.key.decor().suffix().or(" ="));
        render_value_region(&entry.item, indent + INDENT_STEP, out);
    }
    out.push_str(table.trailing().or(""));
}

/// Render a list spelled as a repeated key: `key = a`, `key = b`, ...
fn render_repeated_key(
    entry: &crate::table::TableEntry,
    array: &Array,
    indent: usize,
    out: &mut String,
) {
    for (position, element) in array.elements().iter().enumerate() {
        let prefix = if position == 0 {
            entry.key.decor().prefix().as_str()
        } else {
            element.decor.prefix().as_str()
        };
        push_prefix(prefix, indent, out);
        out.push_str(entry.key.repr().or(entry.key.get()));
        out.push_str(entry.key.decor().suffix().or(" ="));
        render_value_region(&element.item, indent + INDENT_STEP, out);
    }
    out.push_str(array.trailing().or(""));
}

/// Render everything that follows the `=` of an entry.
fn render_value_region(item: &Item, child_indent: usize, out: &mut String) {
    match item {
        Item::None => {}
        Item::Value(value) => out.push_str(&render_scalar(value)),
        Item::Table(table) => render_table(table, child_indent, out),
        Item::Array(array) => render_array(array, child_indent, out),
    }
}

fn render_array(array: &Array, indent: usize, out: &mut String) {
    for element in array.elements() {
        push_prefix(element.decor.prefix().as_str(), indent, out);
        out.push_str(element.decor.suffix().or("="));
        render_value_region(&element.item, indent + INDENT_STEP, out);
    }
    out.push_str(array.trailing().or(""));
}

/// The text of a scalar's value region, including the separating space that
/// canonical output puts after `=`.
fn render_scalar(value: &Value) -> String {
    match value.repr().as_str() {
        Some(raw) => raw.to_string(),
        None => {
            let text = value.as_str();
            if text.is_empty() || text.starts_with('\n') {
                text.to_string()
            } else {
                format!(" {text}")
            }
        }
    }
}

/// Emit an entry separator: the verbatim prefix when known, otherwise a newline
/// (unless this is the very first thing written) plus canonical indentation.
fn push_prefix(prefix: Option<&str>, indent: usize, out: &mut String) {
    match prefix {
        Some(raw) => out.push_str(raw),
        None => {
            if !out.is_empty() {
                out.push('\n');
            }
            for _ in 0..indent {
                out.push(' ');
            }
        }
    }
}

/// Render a whole document body.
pub(crate) fn render_document(root: &Table) -> String {
    let mut out = String::new();
    render_table(root, 0, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{table, value};
    use crate::repr::Key;

    #[test]
    fn canonical_scalars() {
        let mut root = Table::new();
        root.append("name", value("app"));
        root.append("empty", value(""));
        assert_eq!(render_document(&root), "name = app\nempty =");
    }

    #[test]
    fn canonical_nested_tables_indent_by_two() {
        let mut root = Table::new();
        let mut server = Table::new();
        server.append("host", value("localhost"));
        server.append("port", value(8080));
        root.append("server", server);
        assert_eq!(
            render_document(&root),
            "server =\n  host = localhost\n  port = 8080"
        );
    }

    #[test]
    fn canonical_lists_use_bare_syntax() {
        let mut root = Table::new();
        root.append("hosts", vec!["a", "b"]);
        assert_eq!(render_document(&root), "hosts =\n  = a\n  = b");
    }

    #[test]
    fn repeated_key_lists_repeat_the_key() {
        let mut array = Array::with_kind(ArrayKind::RepeatedKey);
        array.push(value("a"));
        array.push(value("b"));
        let mut root = Table::new();
        root.append(Key::new("item"), array);
        assert_eq!(render_document(&root), "item = a\nitem = b");
    }

    #[test]
    fn none_items_are_skipped() {
        let mut root = Table::new();
        root.append("kept", value("1"));
        root.append("dropped", Item::None);
        assert_eq!(render_document(&root), "kept = 1");
    }

    #[test]
    fn empty_table_renders_as_bare_key() {
        let mut root = Table::new();
        root.append("section", table());
        assert_eq!(render_document(&root), "section =");
    }
}
