# Santa Package Manager - CCL Serialization Patterns

## Critical Serialization Rules

### KnownSources Enum Serialization
The `KnownSources` enum uses `camelCase` serialization which requires **lowercase** in CCL files:

```rust
#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum KnownSources {
    Apt,      // Serializes as "apt"
    Aur,      // Serializes as "aur"
    Brew,     // Serializes as "brew"
    Cargo,    // Serializes as "cargo"
    Pacman,   // Serializes as "pacman"
    Scoop,    // Serializes as "scoop"
    Nix,      // Serializes as "nix"
    #[serde(other)]
    Unknown(String),  // Captures any non-matching value
}
```

### CCL File Format Requirements
All source names in CCL configuration files MUST use lowercase:

**✅ Correct:**
```ccl
sources =
  = brew
  = cargo
  = npm
```

**❌ Incorrect:**
```ccl
sources =
  = Brew    # Deserializes as Unknown("Brew")
  = Cargo   # Deserializes as Unknown("Cargo")
  = NPM     # Deserializes as Unknown("NPM")
```

### Validation Impact
When sources deserialize as `Unknown` variants:
- `validate_source_package_compatibility()` fails
- Enum comparison `available_sources.contains_key(configured_source)` returns false
- Error: "Package 'X' is not available from any configured source"

### Known Files Using CCL Sources
1. `data/santa-config.ccl` - DEFAULT_CONFIG embedded in binary
2. `~/.config/santa/config.ccl` - User configuration (if exists)
3. Test fixtures in `src/configuration/watcher.rs`
4. `data/known_packages.ccl` - Package database with source mappings

### Migration Notes
- Old configs may use capitalized source names (invalid)
- Migration to CCL requires lowercase conversion
- YAML configs also need lowercase for source names
