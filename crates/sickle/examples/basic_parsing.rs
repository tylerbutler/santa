//! Basic CCL parsing example using the Model API

use sickle::{parse, Model};

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
    let model = parse(ccl)?;

    // Access simple values
    println!("Application Info:");
    println!("  Name: {}", model.get("name")?.as_str()?);
    println!("  Version: {}", model.get("version")?.as_str()?);
    println!("  Author: {}", model.get("author")?.as_str()?);

    // Navigate nested structures
    println!("\nDatabase Configuration:");
    let db = model.get("database")?;
    if let Ok(map) = db.as_map() {
        for (key, value) in map {
            if let Ok(s) = value.as_str() {
                println!("  {}: {}", key, s);
            } else if let Ok(inner_map) = value.as_map() {
                println!("  {}:", key);
                for (k, v) in inner_map {
                    if let Ok(s) = v.as_str() {
                        println!("    {}: {}", k, s);
                    }
                }
            }
        }
    }

    // Access with path notation
    println!("\nUsing path notation:");
    if let Ok(username) = model.at("database.credentials.username") {
        if let Ok(s) = username.as_str() {
            println!("  DB Username: {}", s);
        }
    }

    // Parse typed values
    println!("\nParsed typed values:");
    if let Ok(port_model) = model.at("database.port") {
        let port: u16 = port_model.parse_value()?;
        println!("  Port (as u16): {}", port);
    }

    Ok(())
}
