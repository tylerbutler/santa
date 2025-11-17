//! Serde deserialization example

#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "serde")]
use sickle::from_str;

#[cfg(feature = "serde")]
#[derive(Deserialize, Debug)]
struct Config {
    name: String,
    version: String,
    author: String,
    database: Database,
}

#[cfg(feature = "serde")]
#[derive(Deserialize, Debug)]
struct Database {
    host: String,
    port: u16,
    credentials: Credentials,
}

#[cfg(feature = "serde")]
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Credentials {
    username: String,
    password: String,
}

#[cfg(feature = "serde")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ccl = r#"
name = Santa Package Manager
version = 0.1.0
author = Tyler Butler

database =
  host = localhost
  port = 5432
  credentials =
    username = admin
    password = secret
"#;

    println!("Deserializing CCL into Rust structs...\n");
    let config: Config = from_str(ccl)?;

    println!("Application: {} v{}", config.name, config.version);
    println!("Author: {}", config.author);
    println!("\nDatabase:");
    println!("  Host: {}", config.database.host);
    println!("  Port: {}", config.database.port);
    println!("  Username: {}", config.database.credentials.username);

    Ok(())
}

#[cfg(not(feature = "serde"))]
fn main() {
    println!("This example requires the 'serde' feature to be enabled.");
    println!("Run with: cargo run --example serde_usage --features serde");
}
