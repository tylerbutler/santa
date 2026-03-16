#[path = "../src/bridge.rs"]
mod bridge;

use serde_json::json;

#[test]
fn simple_string_value() {
    let obj = sickle::load("name = Alice").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"name": "Alice"}));
}

#[test]
fn multiple_keys() {
    let obj = sickle::load("name = Alice\nage = 30").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"name": "Alice", "age": "30"}));
}

#[test]
fn empty_value() {
    let obj = sickle::load("key =").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"key": ""}));
}

#[test]
fn nested_object() {
    let obj = sickle::load("server =\n  host = localhost\n  port = 8080").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"server": {"host": "localhost", "port": "8080"}}));
}

#[test]
fn bare_list() {
    let obj = sickle::load("items =\n  = apple\n  = banana").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"items": ["apple", "banana"]}));
}

#[test]
fn duplicate_key_list() {
    let obj = sickle::load("tag = web\ntag = api").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"tag": ["web", "api"]}));
}

#[test]
fn json_object_to_ccl() {
    let val = json!({"name": "Alice", "age": "30"});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("name = Alice"));
    assert!(ccl.contains("age = 30"));
}

#[test]
fn json_nested_to_ccl() {
    let val = json!({"server": {"host": "localhost", "port": "8080"}});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("server ="));
    assert!(ccl.contains("  host = localhost"));
    assert!(ccl.contains("  port = 8080"));
}

#[test]
fn json_array_to_ccl() {
    let val = json!({"items": ["apple", "banana"]});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("items ="));
    assert!(ccl.contains("  = apple"));
    assert!(ccl.contains("  = banana"));
}

#[test]
fn has_comments_detects_comments() {
    assert!(bridge::has_comments("name = Alice\n/= a comment\nage = 30"));
    assert!(bridge::has_comments("  /= indented comment"));
    assert!(!bridge::has_comments("name = Alice\nage = 30"));
}
