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
    for (key, value) in &db.0 {
        // Check if it's a string (singleton with empty value)
        if value.0.len() == 1 && value.0.values().next().unwrap().0.is_empty() {
            let s = value.0.keys().next().unwrap();
            println!("  {}: {}", key, s);
        } else {
            // It's a nested map
            println!("  {}:", key);
            for (k, v) in &value.0 {
                if v.0.len() == 1 && v.0.values().next().unwrap().0.is_empty() {
                    let s = v.0.keys().next().unwrap();
                    println!("    {}: {}", k, s);
                }
            }
        }
    }

    // Access nested values using chained get() calls
    println!("\nUsing nested access:");
    let username = model.get("database")?.get("credentials")?.get_string("username")?;
    println!("  DB Username: {}", username);

    // Parse typed values using public API
    println!("\nParsed typed values:");
    let port = model.get("database")?.get_int("port")?;
    println!("  Port (as i64): {}", port);

    Ok(())
}
