//! Basic CCL parsing example using the Model API

use sickle::load;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ccl = r#"
/= Application configuration
name = Santa Package Manager
version = 0.1.0
author = Tyler Butler

/= Database settings
database =
  host = localhost
  port = 5432
  credentials =
    username = admin
    password = secret

/= Feature flags
features =
  = hot-reload
  = script-generation
  = multi-platform
"#;

    println!("Parsing CCL document...\n");
    let model = load(ccl)?;

    // Access simple values using public API
    println!("Application Info:");
    println!("  Name: {}", model.get_string("name")?);
    println!("  Version: {}", model.get_string("version")?);
    println!("  Author: {}", model.get_string("author")?);

    // Navigate nested structures using public IndexMap field
    println!("\nDatabase Configuration:");
    let db = model.get("database")?;
    for (key, value) in db.iter() {
        // Check if it's a string (singleton with empty value)
        if value.len() == 1 && value.values().next().unwrap().is_empty() {
            let s = value.keys().next().unwrap();
            println!("  {}: {}", key, s);
        } else {
            // It's a nested map
            println!("  {}:", key);
            for (k, v) in value.iter() {
                if v.len() == 1 && v.values().next().unwrap().is_empty() {
                    let s = v.keys().next().unwrap();
                    println!("    {}: {}", k, s);
                }
            }
        }
    }

    // Access nested values using chained get() calls
    println!("\nUsing nested access:");
    let username = model
        .get("database")?
        .get("credentials")?
        .get_string("username")?;
    println!("  DB Username: {}", username);

    // Parse typed values using public API
    println!("\nParsed typed values:");
    let port = model.get("database")?.get_int("port")?;
    println!("  Port (as i64): {}", port);

    Ok(())
}
