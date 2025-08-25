use mcp_bookmark::config::Config;

#[test]
fn test_multi_index_parsing() {
    // Test single index
    let mut config = Config {
        index_name: Some("single_index".to_string()),
        max_bookmarks: 0,
    };
    
    let indices = config.parse_index_names();
    assert_eq!(indices.len(), 1);
    assert_eq!(indices[0], "single_index");
    assert!(!config.is_multi_index());
    
    // Test multiple indices
    config.index_name = Some("index1,index2,index3".to_string());
    let indices = config.parse_index_names();
    assert_eq!(indices.len(), 3);
    assert_eq!(indices[0], "index1");
    assert_eq!(indices[1], "index2");
    assert_eq!(indices[2], "index3");
    assert!(config.is_multi_index());
    
    // Test with spaces
    config.index_name = Some("index1 , index2 , index3".to_string());
    let indices = config.parse_index_names();
    assert_eq!(indices.len(), 3);
    assert_eq!(indices[0], "index1");
    assert_eq!(indices[1], "index2");
    assert_eq!(indices[2], "index3");
    
    // Test empty string
    config.index_name = Some("".to_string());
    let indices = config.parse_index_names();
    assert_eq!(indices.len(), 0);
    
    // Test None
    config.index_name = None;
    let indices = config.parse_index_names();
    assert_eq!(indices.len(), 0);
}