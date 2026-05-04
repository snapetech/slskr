/// Request/Response filtering and transformation system
///
/// Provides filtering, transformation, and enrichment of API requests and responses
/// for common use cases like field selection, response formatting, and data masking.

use serde_json::{json, Value};
use std::collections::HashMap;

/// Field filter for selective response field inclusion
#[derive(Debug, Clone)]
pub struct FieldFilter {
    pub fields: Vec<String>,
    pub exclude_fields: Vec<String>,
}

impl FieldFilter {
    /// Create a new field filter
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            exclude_fields: Vec::new(),
        }
    }

    /// Add a field to include
    pub fn include(&mut self, field: &str) -> &mut Self {
        self.fields.push(field.to_string());
        self
    }

    /// Add a field to exclude
    pub fn exclude(&mut self, field: &str) -> &mut Self {
        self.exclude_fields.push(field.to_string());
        self
    }

    /// Apply filter to JSON response
    pub fn apply(&self, value: &Value) -> Value {
        if let Value::Object(map) = value {
            let mut filtered = serde_json::Map::new();

            for (key, val) in map {
                // If include list is not empty, only include specified fields
                if !self.fields.is_empty() && !self.fields.contains(key) {
                    continue;
                }

                // Skip excluded fields
                if self.exclude_fields.contains(key) {
                    continue;
                }

                filtered.insert(key.clone(), val.clone());
            }

            Value::Object(filtered)
        } else {
            value.clone()
        }
    }
}

impl Default for FieldFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Response formatting options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    Json,
    Xml,
    Csv,
    Yaml,
}

/// Response formatter
pub struct ResponseFormatter;

impl ResponseFormatter {
    /// Format response body according to specified format
    pub fn format(body: &str, format: ResponseFormat) -> String {
        match format {
            ResponseFormat::Json => body.to_string(),
            ResponseFormat::Xml => Self::json_to_xml(body),
            ResponseFormat::Csv => Self::json_to_csv(body),
            ResponseFormat::Yaml => Self::json_to_yaml(body),
        }
    }

    /// Convert JSON to XML-like format (simplified)
    fn json_to_xml(json_str: &str) -> String {
        let mut xml = String::from("<?xml version=\"1.0\"?>\n<root>\n");

        // Simple conversion - in production, use proper XML library
        if let Ok(value) = serde_json::from_str::<Value>(json_str) {
            xml.push_str(&Self::value_to_xml(&value, 1));
        }

        xml.push_str("</root>");
        xml
    }

    fn value_to_xml(value: &Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let mut xml = String::new();

        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    xml.push_str(&format!("{}<{}>\n", indent_str, key));
                    xml.push_str(&Self::value_to_xml(val, indent + 1));
                    xml.push_str(&format!("{}</{}>\n", indent_str, key));
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    xml.push_str(&Self::value_to_xml(item, indent));
                }
            }
            Value::String(s) => {
                xml.push_str(&format!("{}{}\n", indent_str, s));
            }
            _ => {
                xml.push_str(&format!("{}{}\n", indent_str, value));
            }
        }

        xml
    }

    /// Convert JSON to CSV format (simplified)
    fn json_to_csv(json_str: &str) -> String {
        let mut csv = String::new();

        if let Ok(value) = serde_json::from_str::<Value>(json_str) {
            if let Value::Array(arr) = value {
                if let Some(Value::Object(first)) = arr.first() {
                    // Header row
                    let headers: Vec<_> = first.keys().cloned().collect();
                    csv.push_str(&headers.join(","));
                    csv.push('\n');

                    // Data rows
                    for item in arr {
                        if let Value::Object(map) = item {
                            let values: Vec<String> = headers
                                .iter()
                                .map(|h| {
                                    map.get(h)
                                        .map(|v| v.to_string())
                                        .unwrap_or_default()
                                })
                                .collect();
                            csv.push_str(&values.join(","));
                            csv.push('\n');
                        }
                    }
                }
            }
        }

        csv
    }

    /// Convert JSON to YAML format (simplified)
    fn json_to_yaml(json_str: &str) -> String {
        let mut yaml = String::new();

        if let Ok(value) = serde_json::from_str::<Value>(json_str) {
            yaml.push_str(&Self::value_to_yaml(&value, 0));
        }

        yaml
    }

    fn value_to_yaml(value: &Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let mut yaml = String::new();

        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    yaml.push_str(&format!("{}{}:\n", indent_str, key));
                    yaml.push_str(&Self::value_to_yaml(val, indent + 1));
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    yaml.push_str(&format!("{}- ", indent_str));
                    yaml.push_str(&Self::value_to_yaml(item, indent + 1));
                }
            }
            Value::String(s) => {
                yaml.push_str(&format!("{}\n", s));
            }
            _ => {
                yaml.push_str(&format!("{}\n", value));
            }
        }

        yaml
    }
}

/// Data masking for sensitive fields
pub struct DataMasker {
    pub sensitive_fields: Vec<String>,
    pub mask_char: char,
}

impl DataMasker {
    /// Create a new data masker
    pub fn new() -> Self {
        Self {
            sensitive_fields: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "api_key".to_string(),
            ],
            mask_char: '*',
        }
    }

    /// Mask sensitive fields in JSON response
    pub fn mask(&self, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut masked = serde_json::Map::new();
                for (key, val) in map {
                    if self.sensitive_fields
                        .iter()
                        .any(|sf| key.to_lowercase().contains(sf.as_str()))
                    {
                        // Mask the value
                        masked.insert(
                            key.clone(),
                            Value::String(self.mask_char.to_string().repeat(8)),
                        );
                    } else {
                        masked.insert(key.clone(), self.mask(val));
                    }
                }
                Value::Object(masked)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.mask(v)).collect())
            }
            _ => value.clone(),
        }
    }
}

impl Default for DataMasker {
    fn default() -> Self {
        Self::new()
    }
}

/// Request parser for common query parameters
pub struct QueryParser;

impl QueryParser {
    /// Parse query string into HashMap
    pub fn parse(query_string: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();

        for pair in query_string.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding::decode(key).unwrap_or(key.to_string()),
                    urlencoding::decode(value).unwrap_or(value.to_string()),
                );
            }
        }

        params
    }

    /// Get a parameter value
    pub fn get_param(params: &HashMap<String, String>, key: &str) -> Option<String> {
        params.get(key).cloned()
    }

    /// Get limit parameter (with default and max)
    pub fn get_limit(params: &HashMap<String, String>, default: usize, max: usize) -> usize {
        params
            .get("limit")
            .and_then(|v| v.parse::<usize>().ok())
            .map(|v| v.min(max))
            .unwrap_or(default)
    }

    /// Get offset parameter
    pub fn get_offset(params: &HashMap<String, String>) -> usize {
        params
            .get("offset")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0)
    }
}

/// URL decoding helper (simplified implementation)
mod urlencoding {
    pub fn decode(s: &str) -> Option<String> {
        Some(s.replace("%20", " ").replace("%2B", "+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_filter_include() {
        let mut filter = FieldFilter::new();
        filter.include("name").include("email");

        let data = json!({
            "name": "John",
            "email": "john@example.com",
            "password": "secret"
        });

        let filtered = filter.apply(&data);
        assert!(filtered.get("name").is_some());
        assert!(filtered.get("email").is_some());
        assert!(filtered.get("password").is_none());
    }

    #[test]
    fn test_field_filter_exclude() {
        let mut filter = FieldFilter::new();
        filter.exclude("password");

        let data = json!({
            "name": "John",
            "password": "secret"
        });

        let filtered = filter.apply(&data);
        assert!(filtered.get("name").is_some());
        assert!(filtered.get("password").is_none());
    }

    #[test]
    fn test_data_masker() {
        let masker = DataMasker::new();
        let data = json!({
            "username": "john",
            "password": "secret123"
        });

        let masked = masker.mask(&data);
        assert_eq!(masked["username"], "john");
        assert_ne!(masked["password"], "secret123");
        assert!(masked["password"].as_str().unwrap().contains('*'));
    }

    #[test]
    fn test_response_formatter_json() {
        let json_str = r#"{"name":"John"}"#;
        let formatted = ResponseFormatter::format(json_str, ResponseFormat::Json);
        assert!(formatted.contains("John"));
    }

    #[test]
    fn test_response_formatter_xml() {
        let json_str = r#"{"name":"John"}"#;
        let formatted = ResponseFormatter::format(json_str, ResponseFormat::Xml);
        assert!(formatted.contains("<?xml"));
        assert!(formatted.contains("John"));
    }

    #[test]
    fn test_response_formatter_yaml() {
        let json_str = r#"{"name":"John"}"#;
        let formatted = ResponseFormatter::format(json_str, ResponseFormat::Yaml);
        assert!(formatted.contains("name:"));
        assert!(formatted.contains("John"));
    }

    #[test]
    fn test_query_parser() {
        let query = "name=John&email=john@example.com&limit=10";
        let params = QueryParser::parse(query);

        assert_eq!(QueryParser::get_param(&params, "name"), Some("John".to_string()));
        assert_eq!(QueryParser::get_limit(&params, 20, 100), 10);
    }

    #[test]
    fn test_query_parser_limit_max() {
        let query = "limit=1000";
        let params = QueryParser::parse(query);

        assert_eq!(QueryParser::get_limit(&params, 20, 100), 100);
    }

    #[test]
    fn test_query_parser_offset() {
        let query = "limit=10&offset=20";
        let params = QueryParser::parse(query);

        assert_eq!(QueryParser::get_offset(&params), 20);
    }
}
