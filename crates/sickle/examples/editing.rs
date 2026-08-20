//! Editing a CCL document while keeping its comments and layout.
//!
//! Run with: `cargo run -p sickle --example editing`

use sickle::{table, value, DocumentMut};

const SOURCE: &str = "\
/= Service configuration

/= public name
name = gateway
port   = 8080

server =
  /= bind address
  host = 0.0.0.0
";

fn main() {
    let mut doc: DocumentMut = SOURCE.parse().expect("valid CCL");

    // Checked edits: every path is validated and every failure is structured.
    doc.set_int(["port"], 9090).unwrap();
    doc.set_string(["server", "host"], "127.0.0.1").unwrap();
    doc.set_list(["server", "upstreams"], ["api-1", "api-2"])
        .unwrap();
    doc.insert_comment_before(["server", "upstreams"], "added by the installer")
        .unwrap();

    // Raw edits: terser, but indexing panics and auto-vivifies.
    doc.as_table_mut().insert("tls", table());
    doc["tls"]["enabled"] = value(true);

    // Removing a key removes the comments written directly above it.
    doc.remove(["name"]).unwrap();

    println!("{doc}");
    assert_eq!(
        doc.to_string(),
        "\
/= Service configuration

port   = 9090

server =
  /= bind address
  host = 127.0.0.1
  /= added by the installer
  upstreams =
    = api-1
    = api-2
tls =
  enabled = true
"
    );

    // Untouched lines keep their original spelling (`port   = 9090` above kept
    // its extra spaces); new nodes are written canonically.
    //
    // `fmt` opts into rewriting everything canonically instead.
    let mut canonical: DocumentMut = SOURCE.parse().unwrap();
    canonical.fmt();
    println!("--- canonical ---\n{canonical}");
}
