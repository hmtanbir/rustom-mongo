use std::collections::HashMap;

/// Helper function to parse key-value integer maps from YAML contents.
pub fn parse_yaml_map(yaml_str: &str) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for line in yaml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            if let Ok(parsed_val) = val.trim().parse::<i32>() {
                map.insert(key, parsed_val);
            }
        }
    }
    map
}

// static function to get API rate limit from environment variable.
pub static API_RATE_LIMIT: std::sync::LazyLock<i64> = std::sync::LazyLock::new(|| {
    std::env::var("API_RATE_LIMIT")
        .ok()
        .and_then(|val| val.parse::<i64>().ok())
        .unwrap_or(5)
});
