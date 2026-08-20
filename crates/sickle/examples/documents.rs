//! Reading a CCL document: parsing, navigating, and typed access.
//!
//! Run with: `cargo run -p sickle --example documents`

use sickle::{DocumentMut, Item, PathSegment};

const SOURCE: &str = "\
/= Service configuration
name = gateway
port = 8080
debug = false

server =
  host = 0.0.0.0
  /= reverse proxies, in priority order
  upstreams =
    = api-1
    = api-2

feature = tracing
feature = metrics
";

fn main() {
    let doc: DocumentMut = SOURCE.parse().expect("valid CCL");

    // Parsing is lossless: an untouched document renders back byte-for-byte.
    assert_eq!(doc.to_string(), SOURCE);

    // Checked, typed reads. Every failure names the path that failed.
    println!("name  = {}", doc.get_string(["name"]).unwrap());
    println!("port  = {}", doc.get_int(["port"]).unwrap());
    println!("debug = {}", doc.get_bool(["debug"]).unwrap());
    println!("host  = {}", doc.get_string(["server", "host"]).unwrap());

    // Lists work whichever way CCL spells them: an indented `= item` block ...
    let upstreams = doc.get_list(["server", "upstreams"]).unwrap();
    println!("upstreams = {upstreams:?}");

    // ... or a repeated key.
    let features = doc.get_list(["feature"]).unwrap();
    println!("features  = {features:?}");

    // Numeric segments descend into lists.
    let first = doc
        .get_string([
            PathSegment::key("server"),
            PathSegment::key("upstreams"),
            PathSegment::index(0),
        ])
        .unwrap();
    println!("first upstream = {first}");

    // Enumerate a block's keys without seeing comments or blank lines.
    println!("server keys = {:?}", doc.table_keys(["server"]).unwrap());

    // Walking the raw tree is also available.
    for (key, item) in doc.iter() {
        let kind = match item {
            Item::Value(_) => "scalar",
            Item::Table(_) => "block",
            Item::Array(_) => "list",
            Item::None => "absent",
        };
        println!("{key:>10} : {kind}");
    }

    // Missing paths and wrong types are errors, not panics.
    let err = doc.get_int(["server", "missing"]).unwrap_err();
    println!("error: {err}");
}
