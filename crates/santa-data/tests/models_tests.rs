//! Comprehensive tests for data models

use santa_data::*;
use std::collections::HashMap;

// PackageName tests
#[test]
fn test_package_name_creation() {
    let name = PackageName("bat".to_string());
    assert_eq!(name.to_string(), "bat");
}

#[test]
fn test_package_name_from_string() {
    let name: PackageName = "ripgrep".to_string().into();
    assert_eq!(name.to_string(), "ripgrep");
}

#[test]
fn test_package_name_equality() {
    let name1 = PackageName("bat".to_string());
    let name2 = PackageName("bat".to_string());
    let name3 = PackageName("ripgrep".to_string());

    assert_eq!(name1, name2);
    assert_ne!(name1, name3);
}

#[test]
fn test_package_name_hash() {
    let mut map = HashMap::new();
    let name = PackageName("bat".to_string());
    map.insert(name.clone(), "value");

    assert!(map.contains_key(&name));
}

#[test]
fn test_package_name_serialization() {
    let name = PackageName("bat".to_string());
    let json = serde_json::to_string(&name).unwrap();
    assert_eq!(json, "\"bat\"");
}

#[test]
fn test_package_name_deserialization() {
    let json = "\"bat\"";
    let name: PackageName = serde_json::from_str(json).unwrap();
    assert_eq!(name.to_string(), "bat");
}

// SourceName tests
#[test]
fn test_source_name_creation() {
    let name = SourceName("brew".to_string());
    assert_eq!(name.to_string(), "brew");
}

#[test]
fn test_source_name_equality() {
    let name1 = SourceName("brew".to_string());
    let name2 = SourceName("brew".to_string());
    assert_eq!(name1, name2);
}

// CommandName tests
#[test]
fn test_command_name_creation() {
    let name = CommandName("install".to_string());
    assert_eq!(name.to_string(), "install");
}

#[test]
fn test_command_name_from_string() {
    let name: CommandName = "update".to_string().into();
    assert_eq!(name.to_string(), "update");
}

// KnownSources tests
#[test]
fn test_known_sources_variants() {
    let sources = vec![
        KnownSources::Apt,
        KnownSources::Aur,
        KnownSources::Brew,
        KnownSources::Cargo,
        KnownSources::Flathub,
        KnownSources::Nix,
        KnownSources::Npm,
        KnownSources::Pacman,
        KnownSources::Scoop,
    ];

    for source in sources {
        assert_ne!(source, KnownSources::Unknown("test".to_string()));
    }
}

#[test]
fn test_known_sources_unknown() {
    let source = KnownSources::Unknown("custom".to_string());
    if let KnownSources::Unknown(s) = source {
        assert_eq!(s, "custom");
    } else {
        panic!("Expected Unknown variant");
    }
}

#[test]
fn test_known_sources_equality() {
    assert_eq!(KnownSources::Brew, KnownSources::Brew);
    assert_ne!(KnownSources::Brew, KnownSources::Scoop);
}

#[test]
fn test_known_sources_serialization() {
    let source = KnownSources::Brew;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"brew\"");
}

#[test]
fn test_known_sources_deserialization() {
    let json = "\"brew\"";
    let source: KnownSources = serde_json::from_str(json).unwrap();
    assert_eq!(source, KnownSources::Brew);
}

#[test]
fn test_known_sources_unknown_deserialization() {
    let json = "\"custom-source\"";
    let source: KnownSources = serde_json::from_str(json).unwrap();
    match source {
        KnownSources::Unknown(s) => assert_eq!(s, "custom-source"),
        _ => panic!("Expected Unknown variant"),
    }
}

#[test]
fn test_known_sources_npm_variant() {
    let source = KnownSources::Npm;
    assert_eq!(source, KnownSources::Npm);
    assert_ne!(source, KnownSources::Brew);
}

#[test]
fn test_known_sources_npm_serialization() {
    let source = KnownSources::Npm;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"npm\"");
}

#[test]
fn test_known_sources_npm_deserialization() {
    let json = "\"npm\"";
    let source: KnownSources = serde_json::from_str(json).unwrap();
    assert_eq!(source, KnownSources::Npm);
}

#[test]
fn test_known_sources_flathub_variant() {
    let source = KnownSources::Flathub;
    assert_eq!(source, KnownSources::Flathub);
    assert_ne!(source, KnownSources::Brew);
}

#[test]
fn test_known_sources_flathub_serialization() {
    let source = KnownSources::Flathub;
    let json = serde_json::to_string(&source).unwrap();
    assert_eq!(json, "\"flathub\"");
}

#[test]
fn test_known_sources_flathub_deserialization() {
    let json = "\"flathub\"";
    let source: KnownSources = serde_json::from_str(json).unwrap();
    assert_eq!(source, KnownSources::Flathub);
}

#[test]
fn test_known_sources_hash_map_with_custom() {
    // Test that Unknown variants can be used as HashMap keys
    let mut sources_map: HashMap<KnownSources, String> = HashMap::new();

    sources_map.insert(KnownSources::Brew, "brew package".to_string());
    sources_map.insert(KnownSources::Npm, "npm package".to_string());
    sources_map.insert(
        KnownSources::Unknown("customPM".to_string()),
        "custom package".to_string(),
    );

    assert_eq!(
        sources_map.get(&KnownSources::Brew),
        Some(&"brew package".to_string())
    );
    assert_eq!(
        sources_map.get(&KnownSources::Npm),
        Some(&"npm package".to_string())
    );
    assert_eq!(
        sources_map.get(&KnownSources::Unknown("customPM".to_string())),
        Some(&"custom package".to_string())
    );
}

#[test]
fn test_known_sources_unknown_variant_equality() {
    let custom1 = KnownSources::Unknown("myPM".to_string());
    let custom2 = KnownSources::Unknown("myPM".to_string());
    let custom3 = KnownSources::Unknown("otherPM".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
    assert_ne!(custom1, KnownSources::Brew);
}

// OS tests
#[test]
fn test_os_variants() {
    let oses = vec![OS::Macos, OS::Linux, OS::Windows];
    assert_eq!(oses.len(), 3);
}

#[test]
fn test_os_serialization() {
    assert_eq!(serde_json::to_string(&OS::Macos).unwrap(), "\"macos\"");
    assert_eq!(serde_json::to_string(&OS::Linux).unwrap(), "\"linux\"");
    assert_eq!(serde_json::to_string(&OS::Windows).unwrap(), "\"windows\"");
}

#[test]
fn test_os_deserialization() {
    assert_eq!(serde_json::from_str::<OS>("\"macos\"").unwrap(), OS::Macos);
    assert_eq!(serde_json::from_str::<OS>("\"linux\"").unwrap(), OS::Linux);
    assert_eq!(
        serde_json::from_str::<OS>("\"windows\"").unwrap(),
        OS::Windows
    );
}

// Arch tests
#[test]
fn test_arch_variants() {
    let arches = vec![Arch::X64, Arch::Aarch64];
    assert_eq!(arches.len(), 2);
}

#[test]
fn test_arch_serialization() {
    assert_eq!(serde_json::to_string(&Arch::X64).unwrap(), "\"x64\"");
    assert_eq!(
        serde_json::to_string(&Arch::Aarch64).unwrap(),
        "\"aarch64\""
    );
}

// Distro tests
#[test]
fn test_distro_variants() {
    let distros = vec![Distro::None, Distro::ArchLinux, Distro::Ubuntu];
    assert_eq!(distros.len(), 3);
}

#[test]
fn test_distro_serialization() {
    assert_eq!(serde_json::to_string(&Distro::None).unwrap(), "\"none\"");
    assert_eq!(
        serde_json::to_string(&Distro::ArchLinux).unwrap(),
        "\"archLinux\""
    );
    assert_eq!(
        serde_json::to_string(&Distro::Ubuntu).unwrap(),
        "\"ubuntu\""
    );
}

// Platform tests
#[test]
fn test_platform_default() {
    let platform = Platform::default();
    assert_eq!(platform.os, OS::Linux);
    assert_eq!(platform.arch, Arch::X64);
    assert_eq!(platform.distro, None);
}

#[test]
fn test_platform_creation() {
    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    assert_eq!(platform.os, OS::Macos);
    assert_eq!(platform.arch, Arch::Aarch64);
}

#[test]
fn test_platform_with_distro() {
    let platform = Platform {
        os: OS::Linux,
        arch: Arch::X64,
        distro: Some(Distro::Ubuntu),
    };

    assert_eq!(platform.distro, Some(Distro::Ubuntu));
}

#[test]
fn test_platform_display_without_distro() {
    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    let display = format!("{}", platform);
    assert!(display.contains("Macos"));
    assert!(display.contains("Aarch64"));
}

#[test]
fn test_platform_display_with_distro() {
    let platform = Platform {
        os: OS::Linux,
        arch: Arch::X64,
        distro: Some(Distro::Ubuntu),
    };

    let display = format!("{}", platform);
    assert!(display.contains("Linux"));
    assert!(display.contains("X64"));
    assert!(display.contains("Ubuntu"));
}

#[test]
fn test_platform_equality() {
    let p1 = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    let p2 = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    let p3 = Platform {
        os: OS::Linux,
        arch: Arch::X64,
        distro: None,
    };

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
}

#[test]
fn test_platform_serialization() {
    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: Some(Distro::Ubuntu),
    };

    let json = serde_json::to_string(&platform).unwrap();
    assert!(json.contains("macos"));
    assert!(json.contains("aarch64"));
}

#[test]
fn test_platform_deserialization() {
    let json = r#"{"os":"macos","arch":"aarch64","distro":"ubuntu"}"#;
    let platform: Platform = serde_json::from_str(json).unwrap();

    assert_eq!(platform.os, OS::Macos);
    assert_eq!(platform.arch, Arch::Aarch64);
    assert_eq!(platform.distro, Some(Distro::Ubuntu));
}

// PackageData tests
#[test]
fn test_package_data_new() {
    let data = PackageData::new("bat");
    assert_eq!(data.name, Some("bat".to_string()));
    assert!(data.before.is_none());
    assert!(data.after.is_none());
    assert!(data.pre.is_none());
    assert!(data.post.is_none());
}

#[test]
fn test_package_data_with_hooks() {
    let data = PackageData {
        name: Some("bat".to_string()),
        before: Some("echo before".to_string()),
        after: Some("echo after".to_string()),
        pre: Some("pre-install".to_string()),
        post: Some("post-install".to_string()),
    };

    assert_eq!(data.before, Some("echo before".to_string()));
    assert_eq!(data.after, Some("echo after".to_string()));
    assert_eq!(data.pre, Some("pre-install".to_string()));
    assert_eq!(data.post, Some("post-install".to_string()));
}

#[test]
fn test_package_data_equality() {
    let data1 = PackageData::new("bat");
    let data2 = PackageData::new("bat");
    let data3 = PackageData::new("ripgrep");

    assert_eq!(data1, data2);
    assert_ne!(data1, data3);
}

#[test]
fn test_package_data_serialization() {
    let data = PackageData::new("bat");
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("bat"));
}

#[test]
fn test_package_data_deserialization() {
    let json = r#"{"name":"bat","before":null,"after":null,"pre":null,"post":null}"#;
    let data: PackageData = serde_json::from_str(json).unwrap();
    assert_eq!(data.name, Some("bat".to_string()));
}

#[test]
fn test_package_data_partial_fields() {
    let json = r#"{"name":"bat","pre":"pre-cmd"}"#;
    let data: PackageData = serde_json::from_str(json).unwrap();
    assert_eq!(data.name, Some("bat".to_string()));
    assert_eq!(data.pre, Some("pre-cmd".to_string()));
    assert!(data.post.is_none());
}

// PackageDataList tests
#[test]
fn test_package_data_list_creation() {
    let mut list: PackageDataList = HashMap::new();
    let mut source_data = HashMap::new();
    source_data.insert(KnownSources::Brew, Some(PackageData::new("bat")));

    list.insert("bat".to_string(), source_data);

    assert!(list.contains_key("bat"));
}

#[test]
fn test_package_data_list_multiple_sources() {
    let mut list: PackageDataList = HashMap::new();
    let mut source_data = HashMap::new();
    source_data.insert(KnownSources::Brew, Some(PackageData::new("gh")));
    source_data.insert(KnownSources::Scoop, Some(PackageData::new("bat")));

    list.insert("bat".to_string(), source_data);

    let bat_data = list.get("bat").unwrap();
    assert!(bat_data.contains_key(&KnownSources::Brew));
    assert!(bat_data.contains_key(&KnownSources::Scoop));
}
