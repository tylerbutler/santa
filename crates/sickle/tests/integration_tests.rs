//! Contract tests for the public Sickle API.
//!
//! These lock the names, shapes, and behaviors the crate promises: parsing and
//! byte-for-byte round-tripping, raw mutation, checked path edits, structured
//! errors, trivia preservation, configurable behaviors, and Serde.

use serde::{Deserialize, Serialize};
use sickle::{
    array, compose, table, value, Array, ArrayKind, BoolBehavior, CrlfBehavior, DelimiterStrategy,
    DocumentMut, EditError, Entry, GetError, Item, Key, ListBehavior, Options, PathSegment,
    SpacingBehavior, TabBehavior, Table, Value,
};
use std::str::FromStr;

// ============================================================================
// Parsing and rendering
// ============================================================================

#[test]
fn from_str_and_display_are_the_parse_render_pair() {
    let source = "name = api\n";
    let doc = DocumentMut::from_str(source).unwrap();
    assert_eq!(doc.to_string(), source);
    assert_eq!(sickle::parse(source).unwrap().to_string(), source);
}

#[test]
fn unmodified_documents_round_trip_byte_for_byte() {
    let sources = [
        "",
        "a = 1",
        "a = 1\n",
        "a=1\n",
        "a   =   1   \n",
        "\n\n/= leading\n\nname = api\n\n",
        "root =\n  child =\n    leaf = 1\n",
        "list =\n  = one\n  = two\n",
        "dup = 1\ndup = 2\n",
        "script =\n  #!/bin/sh\n  echo hi\n",
        "config =\r\n  host = h\r\n",
        "\ttabbed = 1\n",
        "no_delimiter\nnext = 1\n",
    ];
    for source in sources {
        let doc = DocumentMut::parse(source).unwrap();
        assert_eq!(doc.to_string(), source, "source {source:?}");
    }
}

#[test]
fn parse_errors_carry_positions() {
    // Nesting past the recursion limit is the one structural failure CCL has.
    let mut input = String::from("a = b");
    for _ in 0..80 {
        let indented: String = input.lines().map(|l| format!("  {l}\n")).collect();
        input = format!("outer =\n{indented}");
    }
    let err = DocumentMut::parse(&input).unwrap_err();
    assert!(err.message().contains("nesting depth"));
    let (line, column) = err.line_column();
    assert!(line >= 1 && column >= 1);
}

#[test]
fn documents_expose_their_root_table() {
    let doc: DocumentMut = "a = 1\nb =\n  c = 2\n".parse().unwrap();
    assert_eq!(doc.len(), 2);
    assert!(!doc.is_empty());
    assert_eq!(doc.root_keys(), vec!["a", "b"]);
    assert_eq!(doc.as_table().get("a").unwrap().as_str(), Some("1"));
    assert_eq!(doc.trailing().or(""), "\n");

    let keys: Vec<_> = doc.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, vec!["a", "b"]);
}

// ============================================================================
// The tree: items, tables, arrays, values
// ============================================================================

#[test]
fn scalars_keep_text_and_offer_checked_views() {
    let doc: DocumentMut = "n = 42\nf = 3.5\nb = true\ns = hello\n".parse().unwrap();
    let n = doc.get(["n"]).unwrap().as_value().unwrap();
    assert_eq!(n.as_str(), "42");
    assert_eq!(n.as_integer(), Some(42));
    assert_eq!(n.as_bool(), None);
    assert!((doc.get(["f"]).unwrap().as_float().unwrap() - 3.5).abs() < f64::EPSILON);
    assert_eq!(doc.get(["b"]).unwrap().as_bool(), Some(true));
    assert_eq!(doc.get(["s"]).unwrap().as_integer(), None);
}

#[test]
fn nested_blocks_parse_as_tables() {
    let doc: DocumentMut = "server =\n  host = h\n  port = 1\n".parse().unwrap();
    let server = doc.get(["server"]).unwrap().as_table().unwrap();
    assert_eq!(server.len(), 2);
    assert_eq!(server.unique_keys(), vec!["host", "port"]);
}

#[test]
fn both_list_spellings_read_as_lists() {
    let bare: DocumentMut = "hosts =\n  = a\n  = b\n".parse().unwrap();
    assert!(bare.get(["hosts"]).unwrap().is_array());
    assert_eq!(bare.get_list(["hosts"]).unwrap(), vec!["a", "b"]);

    let repeated: DocumentMut = "host = a\nhost = b\n".parse().unwrap();
    assert_eq!(repeated.as_table().count("host"), 2);
    assert_eq!(repeated.get_list(["host"]).unwrap(), vec!["a", "b"]);
}

#[test]
fn duplicate_keys_keep_their_source_order() {
    let doc: DocumentMut = "x = 1\ny = 2\nx = 3\n".parse().unwrap();
    let table = doc.as_table();
    assert_eq!(table.len(), 3);
    assert_eq!(table.keys().collect::<Vec<_>>(), vec!["x", "y", "x"]);
    assert_eq!(table.get("x").unwrap().as_str(), Some("1"));
    let all: Vec<_> = table
        .get_all("x")
        .into_iter()
        .map(|i| i.as_str().unwrap())
        .collect();
    assert_eq!(all, vec!["1", "3"]);
}

#[test]
fn repeated_blocks_compose() {
    let doc: DocumentMut = "user =\n  id = 1\nuser =\n  name = ada\n".parse().unwrap();
    let user = doc.as_table().get_composed("user").unwrap();
    assert_eq!(user.get_string(["id"]).unwrap(), "1");
    assert_eq!(user.get_string(["name"]).unwrap(), "ada");
}

#[test]
fn compose_is_a_monoid_over_blocks() {
    let mut left = Table::new();
    left.append("a", value("1"));
    let mut right = Table::new();
    right.append("b", value("2"));

    let merged = compose(Item::Table(left.clone()), Item::Table(right));
    assert_eq!(merged.get_string(["a"]).unwrap(), "1");
    assert_eq!(merged.get_string(["b"]).unwrap(), "2");

    // The empty node is the identity element.
    assert_eq!(
        compose(Item::None, Item::Table(left.clone())),
        Item::Table(left)
    );
}

#[test]
fn item_constructors_and_conversions() {
    assert!(value("x").is_value());
    assert!(table().is_table());
    assert!(array().is_array());
    assert!(Item::None.is_none());

    let item: Item = vec!["a", "b"].into();
    assert_eq!(item.as_array().unwrap().as_strings(), Some(vec!["a", "b"]));
    assert_eq!(Item::from(7i64).as_integer(), Some(7));
    assert_eq!(Value::from(true).as_str(), "true");
}

// ============================================================================
// Raw editing
// ============================================================================

#[test]
fn tables_support_insert_append_and_remove() {
    let mut table = Table::new();
    table.append("a", value("1"));
    table.append("a", value("2"));
    assert_eq!(table.count("a"), 2);

    table.insert("a", value("3"));
    assert_eq!(table.count("a"), 1);
    assert_eq!(table.get("a").unwrap().as_str(), Some("3"));

    assert!(table.remove("a").is_some());
    assert!(table.remove("a").is_none());
    assert!(table.is_empty());
}

#[test]
fn entry_api_covers_occupied_and_vacant() {
    let mut table = Table::new();
    table.entry("server").or_insert(table_node());
    assert!(matches!(table.entry("server"), Entry::Occupied(_)));
    assert!(matches!(table.entry("other"), Entry::Vacant(_)));
    assert_eq!(table.entry("server").key(), "server");

    match table.entry("server") {
        Entry::Occupied(entry) => {
            let removed = entry.remove();
            assert!(removed.is_table());
        }
        Entry::Vacant(_) => panic!("expected an occupied entry"),
    }
    assert!(table.is_empty());
}

fn table_node() -> Item {
    table()
}

#[test]
fn indexing_reads_and_auto_vivifies() {
    let mut doc = DocumentMut::new();
    doc["server"]["host"] = value("localhost");
    assert_eq!(doc["server"]["host"].as_str(), Some("localhost"));
    assert_eq!(doc.to_string(), "server =\n  host = localhost");
}

#[test]
#[should_panic(expected = "not found")]
fn indexing_a_missing_key_panics() {
    let doc: DocumentMut = "a = 1\n".parse().unwrap();
    let _ = &doc["missing"];
}

#[test]
fn arrays_support_push_insert_and_remove() {
    let mut array = Array::new();
    array.push(value("a"));
    array.push(value("c"));
    array.insert(1, value("b"));
    assert_eq!(array.as_strings(), Some(vec!["a", "b", "c"]));
    assert_eq!(array.remove(1).as_str(), Some("b"));
    assert_eq!(array[1].as_str(), Some("c"));
}

#[test]
fn array_kind_selects_the_list_spelling() {
    let mut doc = DocumentMut::new();
    let mut repeated = Array::with_kind(ArrayKind::RepeatedKey);
    repeated.push(value("a"));
    repeated.push(value("b"));
    doc.as_table_mut().insert("item", repeated);
    assert_eq!(doc.to_string(), "item = a\nitem = b");

    let mut doc = DocumentMut::new();
    doc.as_table_mut().insert("item", vec!["a", "b"]);
    assert_eq!(doc.to_string(), "item =\n  = a\n  = b");
}

// ============================================================================
// Checked editing
// ============================================================================

#[test]
fn checked_reads_cover_every_scalar_type() {
    let doc: DocumentMut = "s = text\ni = 5\nf = 2.5\nb = false\nl =\n  = a\n"
        .parse()
        .unwrap();
    assert_eq!(doc.get_string(["s"]).unwrap(), "text");
    assert_eq!(doc.get_int(["i"]).unwrap(), 5);
    assert!((doc.get_float(["f"]).unwrap() - 2.5).abs() < f64::EPSILON);
    assert!(!doc.get_bool(["b"]).unwrap());
    assert_eq!(doc.get_list(["l"]).unwrap(), vec!["a"]);
    assert_eq!(doc.value_get::<u8, _>(["i"]).unwrap(), 5u8);
}

#[test]
fn checked_writes_create_and_replace() {
    let mut doc = DocumentMut::new();
    doc.set_string(["a", "b"], "x").unwrap();
    doc.set_int(["a", "n"], 3).unwrap();
    doc.set_float(["a", "f"], 1.5).unwrap();
    doc.set_bool(["a", "flag"], true).unwrap();
    doc.set_list(["a", "items"], ["p", "q"]).unwrap();

    assert_eq!(doc.get_string(["a", "b"]).unwrap(), "x");
    assert_eq!(doc.get_int(["a", "n"]).unwrap(), 3);
    assert_eq!(doc.get_list(["a", "items"]).unwrap(), vec!["p", "q"]);

    doc.set_string(["a", "b"], "y").unwrap();
    assert_eq!(doc.get_string(["a", "b"]).unwrap(), "y");
}

#[test]
fn index_segments_walk_lists() {
    let mut doc: DocumentMut = "l =\n  = a\n  = b\n".parse().unwrap();
    let path = [PathSegment::key("l"), PathSegment::index(1)];
    assert_eq!(doc.get_string(path).unwrap(), "b");
    doc.set(path, value("z")).unwrap();
    assert_eq!(doc.get_list(["l"]).unwrap(), vec!["a", "z"]);
    assert_eq!(doc.to_string(), "l =\n  = a\n  = z\n");
}

#[test]
fn checked_errors_are_structured_and_carry_paths() {
    let doc: DocumentMut = "a =\n  b = 1\n".parse().unwrap();

    let empty: [PathSegment<'_>; 0] = [];
    assert!(matches!(doc.get(empty), Err(GetError::EmptyPath)));

    match doc.get_string(["a", "missing"]) {
        Err(GetError::MissingKey { path, key }) => {
            assert_eq!(path, "a.missing");
            assert_eq!(key, "missing");
        }
        other => panic!("unexpected: {other:?}"),
    }

    match doc.get_string(["a"]) {
        Err(GetError::TypeMismatch { expected, .. }) => {
            assert_eq!(expected, sickle::ExpectedType::String)
        }
        other => panic!("unexpected: {other:?}"),
    }

    match doc.get_int(["a", "b"]) {
        Ok(v) => assert_eq!(v, 1),
        Err(e) => panic!("unexpected: {e}"),
    }

    let doc: DocumentMut = "a = xyz\n".parse().unwrap();
    match doc.get_int(["a"]) {
        Err(GetError::InvalidValue {
            expected, value, ..
        }) => {
            assert_eq!(expected, sickle::ExpectedType::Integer);
            assert_eq!(value, "xyz");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn edit_errors_are_structured() {
    let mut doc: DocumentMut = "a = 1\nl =\n  = x\n".parse().unwrap();

    let empty: [PathSegment<'_>; 0] = [];
    assert!(matches!(
        doc.set(empty, value("x")),
        Err(EditError::EmptyPath)
    ));
    assert!(matches!(
        doc.set_string(["a", "b"], "x"),
        Err(EditError::NotATable { .. })
    ));
    assert!(matches!(
        doc.set([PathSegment::key("l"), PathSegment::index(9)], value("x")),
        Err(EditError::IndexOutOfBounds { .. })
    ));
    assert!(matches!(
        doc.remove(["nope"]),
        Err(EditError::MissingKey { .. })
    ));
    assert!(matches!(
        doc.insert_comment_before(["a"], "bad\ncomment"),
        Err(EditError::InvalidComment { .. })
    ));
}

#[test]
fn table_keys_enumerates_a_block() {
    let doc: DocumentMut = "a =\n  x = 1\n  y = 2\n  x = 3\n".parse().unwrap();
    assert_eq!(doc.table_keys(["a"]).unwrap(), vec!["x", "y"]);
    let empty: [PathSegment<'_>; 0] = [];
    assert_eq!(doc.table_keys(empty).unwrap(), vec!["a"]);
}

#[test]
fn remove_returns_the_removed_node() {
    let mut doc: DocumentMut = "a = 1\nb = 2\n".parse().unwrap();
    let removed = doc.remove(["a"]).unwrap();
    assert_eq!(removed.as_str(), Some("1"));
    assert_eq!(doc.to_string(), "b = 2\n");
}

// ============================================================================
// Trivia preservation
// ============================================================================

#[test]
fn editing_a_value_preserves_neighboring_trivia() {
    let source = "/= header\n\n/= about name\nname = api\n\nserver =\n  /= inner\n  port = 80\n\n/= footer\n";
    let mut doc: DocumentMut = source.parse().unwrap();
    doc.set_int(["server", "port"], 8080).unwrap();
    doc.set_string(["name"], "gateway").unwrap();

    let expected = "/= header\n\n/= about name\nname = gateway\n\nserver =\n  /= inner\n  port = 8080\n\n/= footer\n";
    assert_eq!(doc.to_string(), expected);
}

#[test]
fn new_entries_use_canonical_formatting() {
    let mut doc: DocumentMut = "a   =   1\n".parse().unwrap();
    doc.set_string(["b"], "2").unwrap();
    assert_eq!(doc.to_string(), "a   =   1\nb = 2\n");
}

#[test]
fn comments_can_be_inserted_and_appended() {
    let mut doc: DocumentMut = "name = api\nport = 80\n".parse().unwrap();
    doc.insert_comment_before(["port"], "listening port")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "name = api\n/= listening port\nport = 80\n"
    );

    doc.push_comment("end of file").unwrap();
    assert!(doc.to_string().ends_with("/= end of file\n"));

    doc.insert_comment_at(0, "generated").unwrap();
    assert!(doc.to_string().starts_with("/= generated\nname = api\n"));
}

#[test]
fn comments_inside_nested_blocks_keep_their_indentation() {
    let mut doc: DocumentMut = "server =\n  host = h\n  port = 1\n".parse().unwrap();
    doc.insert_comment_before(["server", "port"], "the port")
        .unwrap();
    assert_eq!(
        doc.to_string(),
        "server =\n  host = h\n  /= the port\n  port = 1\n"
    );
}

#[test]
fn fmt_rewrites_a_document_canonically() {
    let mut doc: DocumentMut = "a   =   1\nb=2\n".parse().unwrap();
    doc.fmt();
    assert_eq!(doc.to_string(), "a = 1\nb = 2");
}

#[test]
fn fmt_preserves_comments_and_blank_lines() {
    let mut doc: DocumentMut =
        "/= header\nsection   =\n    /= nested\n    value=1\n\n/= footer\nother = 2\n"
            .parse()
            .unwrap();
    doc.fmt();
    assert_eq!(
        doc.to_string(),
        "/= header\nsection =\n  /= nested\n  value = 1\n\n/= footer\nother = 2"
    );
}

#[test]
fn key_and_decor_expose_formatting() {
    let doc: DocumentMut = "  spaced   = 1\n".parse().unwrap();
    let key = doc.as_table().key("spaced").unwrap();
    assert_eq!(key.get(), "spaced");
    assert_eq!(key.repr().or(""), "spaced");
    assert_eq!(key.decor().prefix().or(""), "  ");
    assert_eq!(key.decor().suffix().or(""), "   =");

    let plain = Key::new("spaced");
    assert_eq!(&plain, key, "keys compare by text, not formatting");
}

// ============================================================================
// Options
// ============================================================================

#[test]
fn spacing_and_delimiter_behavior_is_configurable() {
    let strict = Options::new().with_spacing(SpacingBehavior::Strict);
    let doc = DocumentMut::parse_with("a=1\n", &strict).unwrap();
    assert!(doc.as_table().contains_key("a=1"));

    let spaced = Options::new().with_delimiter(DelimiterStrategy::PreferSpaced);
    let doc = DocumentMut::parse_with("https://x.com?q=1 = result\n", &spaced).unwrap();
    assert_eq!(doc.get_string(["https://x.com?q=1"]).unwrap(), "result");
}

#[test]
fn tab_and_crlf_behavior_is_configurable() {
    let to_spaces = Options::new().with_tabs(TabBehavior::ToSpaces);
    let doc = DocumentMut::parse_with("a =\tvalue\n", &to_spaces).unwrap();
    assert_eq!(doc.get_string(["a"]).unwrap(), "value");

    let normalize = Options::new().with_crlf(CrlfBehavior::NormalizeToLf);
    let doc = DocumentMut::parse_with("a = 1\r\n", &normalize).unwrap();
    assert_eq!(doc.to_string(), "a = 1\n");
}

#[test]
fn boolean_and_list_behavior_is_configurable() {
    let doc: DocumentMut = "flag = yes\nsingle = one\n".parse().unwrap();
    assert!(doc.get_bool(["flag"]).is_err());
    assert!(doc.get_list(["single"]).is_err());

    let lenient = Options::new()
        .with_bool(BoolBehavior::Lenient)
        .with_list(ListBehavior::Coerce);
    let doc = DocumentMut::parse_with("flag = yes\nsingle = one\n", &lenient).unwrap();
    assert!(doc.get_bool(["flag"]).unwrap());
    assert_eq!(doc.get_list(["single"]).unwrap(), vec!["one"]);
}

// ============================================================================
// Serde
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct AppConfig {
    name: String,
    server: Server,
    hosts: Vec<String>,
    enabled: bool,
}

#[test]
fn serde_round_trips_nested_data() {
    let config = AppConfig {
        name: "app".into(),
        server: Server {
            host: "localhost".into(),
            port: 8080,
        },
        hosts: vec!["a".into(), "b".into()],
        enabled: true,
    };

    let text = sickle::ser::to_string(&config).unwrap();
    let parsed: AppConfig = sickle::de::from_str(&text).unwrap();
    assert_eq!(config, parsed);
}

#[test]
fn serde_reads_from_a_parsed_document() {
    let doc: DocumentMut = "host = h\nport = 1\n".parse().unwrap();
    let server: Server = sickle::de::from_document(&doc).unwrap();
    assert_eq!(server.port, 1);

    let item = doc.get(["port"]).unwrap();
    let port: u16 = sickle::de::from_item(item).unwrap();
    assert_eq!(port, 1);
}

#[test]
fn serde_honors_parser_options() {
    #[derive(Deserialize)]
    struct Doc {
        enabled: bool,
    }
    assert!(sickle::de::from_str::<Doc>("enabled = yes\n").is_err());

    let lenient = Options::new().with_bool(BoolBehavior::Lenient);
    let doc: Doc = sickle::de::from_str_with("enabled = yes\n", &lenient).unwrap();
    assert!(doc.enabled);
}

#[test]
fn typed_editing_composes_with_comment_preservation() {
    // The idiomatic replacement for the old serialize-and-merge document API:
    // read typed, mutate, then write the value back through the checked API.
    let source = "/= keep me\nhost = old\nport = 1\n";
    let mut doc: DocumentMut = source.parse().unwrap();
    let mut server: Server = sickle::de::from_document(&doc).unwrap();

    server.host = "new".into();
    doc.set_string(["host"], &server.host).unwrap();

    assert_eq!(doc.to_string(), "/= keep me\nhost = new\nport = 1\n");
}

#[test]
fn serde_errors_are_reported_through_the_unified_error() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Doc {
        needed: String,
    }
    let err = sickle::de::from_str::<Doc>("other = 1\n").unwrap_err();
    assert!(matches!(err, sickle::Error::Deserialize(_)));
    assert!(err.to_string().starts_with("deserialize error"));
}
