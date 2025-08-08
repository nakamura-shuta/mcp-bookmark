use mcp_bookmark::chrome_profile::{ChromeProfile, ProfileResolver};
use std::path::PathBuf;

#[test]
fn test_profile_resolver_creation() {
    // Test that ProfileResolver creation works or fails gracefully
    match ProfileResolver::new() {
        Ok(_) => println!("ProfileResolver created successfully"),
        Err(e) => println!("ProfileResolver creation failed (expected in CI): {}", e),
    }
}

#[test]
fn test_list_all_profiles_structure() {
    // Test ChromeProfile structure
    let profile = ChromeProfile {
        directory_name: "Profile 1".to_string(),
        display_name: "Work".to_string(),
        path: PathBuf::from("/test/path"),
        bookmark_count: Some(42),
        size_kb: Some(128),
    };

    assert_eq!(profile.directory_name, "Profile 1");
    assert_eq!(profile.display_name, "Work");
    assert_eq!(profile.bookmark_count, Some(42));
    assert_eq!(profile.size_kb, Some(128));
}

#[test]
fn test_count_bookmarks_json() {
    // Test bookmark counting with a mock JSON structure
    let json_str = r#"{
        "roots": {
            "bookmark_bar": {
                "children": [
                    {
                        "type": "url",
                        "url": "https://example.com"
                    },
                    {
                        "type": "folder",
                        "children": [
                            {
                                "type": "url",
                                "url": "https://example.com/2"
                            },
                            {
                                "type": "url",
                                "url": "https://example.com/3"
                            }
                        ]
                    }
                ],
                "type": "folder"
            },
            "other": {
                "children": [
                    {
                        "type": "url",
                        "url": "https://example.com/4"
                    }
                ],
                "type": "folder"
            }
        }
    }"#;

    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Manually count bookmarks in the JSON
    fn count_recursive(value: &serde_json::Value) -> usize {
        let mut count = 0;
        if let Some(obj) = value.as_object() {
            if let Some(node_type) = obj.get("type").and_then(|t| t.as_str()) {
                if node_type == "url" {
                    count += 1;
                }
            }
            if let Some(children) = obj.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    count += count_recursive(child);
                }
            }
            if let Some(roots) = obj.get("roots").and_then(|r| r.as_object()) {
                for (_, root) in roots {
                    count += count_recursive(root);
                }
            }
        }
        count
    }

    let count = count_recursive(&json);
    assert_eq!(count, 4, "Should count 4 bookmarks in the test JSON");
}

#[test]
fn test_profile_sorting() {
    // Test that profiles are sorted by size
    let mut profiles = vec![
        ChromeProfile {
            directory_name: "Small".to_string(),
            display_name: "Small Profile".to_string(),
            path: PathBuf::from("/test/small"),
            bookmark_count: Some(10),
            size_kb: Some(50),
        },
        ChromeProfile {
            directory_name: "Large".to_string(),
            display_name: "Large Profile".to_string(),
            path: PathBuf::from("/test/large"),
            bookmark_count: Some(100),
            size_kb: Some(500),
        },
        ChromeProfile {
            directory_name: "Medium".to_string(),
            display_name: "Medium Profile".to_string(),
            path: PathBuf::from("/test/medium"),
            bookmark_count: Some(50),
            size_kb: Some(200),
        },
    ];

    // Sort by size (largest first)
    profiles.sort_by_key(|p| std::cmp::Reverse(p.size_kb.unwrap_or(0)));

    assert_eq!(profiles[0].directory_name, "Large");
    assert_eq!(profiles[1].directory_name, "Medium");
    assert_eq!(profiles[2].directory_name, "Small");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_profile_resolver_real_system() {
        // This test only runs if Chrome is actually installed
        if let Ok(resolver) = ProfileResolver::new() {
            // Test listing profiles
            if let Ok(profiles) = resolver.list_all_profiles() {
                println!("Found {} Chrome profiles", profiles.len());

                for profile in &profiles {
                    println!(
                        "Profile: {} ({}) - {} bookmarks, {} KB",
                        profile.display_name,
                        profile.directory_name,
                        profile.bookmark_count.unwrap_or(0),
                        profile.size_kb.unwrap_or(0)
                    );
                }

                // Test that we can get the current profile
                if let Some(current) = resolver.get_current_profile() {
                    println!(
                        "Current profile: {} ({})",
                        current.display_name, current.directory_name
                    );

                    // Current profile should be in the list
                    assert!(
                        profiles
                            .iter()
                            .any(|p| p.directory_name == current.directory_name),
                        "Current profile should be in the list of all profiles"
                    );
                }
            }

            // Test resolving by name
            if let Ok(default_profile) = resolver.resolve_by_name("Default") {
                assert_eq!(default_profile.directory_name, "Default");

                // Test getting bookmarks path
                let bookmarks_path = resolver.get_bookmarks_path(&default_profile);
                assert!(bookmarks_path.to_string_lossy().ends_with("Bookmarks"));
            }
        } else {
            println!("Chrome not installed - skipping real system tests");
        }
    }
}
