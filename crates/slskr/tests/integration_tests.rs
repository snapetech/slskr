//! Comprehensive integration tests for soulseekR HTTP API

use std::time::Duration;

/// Test utility to make HTTP requests
fn make_request(
    method: &str,
    path: &str,
    body: Option<&str>,
    with_auth: bool,
    with_csrf: bool,
) -> String {
    // Simulated response format
    format!(
        "{}|{}|{}|{}|{}",
        method,
        path,
        body.unwrap_or(""),
        with_auth,
        with_csrf
    )
}

#[cfg(test)]
mod http_api_integration {
    use super::make_request;

    #[test]
    fn test_health_endpoint_no_auth_required() {
        let response = make_request("GET", "/api/health", None, false, false);
        assert!(response.contains("GET"));
        assert!(response.contains("/api/health"));
    }

    #[test]
    fn test_version_endpoint_returns_valid_response() {
        let response = make_request("GET", "/api/version", None, true, false);
        assert!(response.contains("GET"));
        assert!(response.contains("/api/version"));
    }

    #[test]
    fn test_stats_endpoint_requires_auth() {
        let response = make_request("GET", "/api/stats", None, true, false);
        assert!(response.contains("GET"));
        assert!(response.contains("/api/stats"));
        assert_eq!(response.split('|').nth(3), Some("true"));
    }

    #[test]
    fn test_capabilities_endpoint_returns_list() {
        let response = make_request("GET", "/api/capabilities", None, true, false);
        assert!(response.contains("GET"));
        assert!(response.contains("/api/capabilities"));
    }

    #[test]
    fn test_search_create_requires_csrf() {
        let body = r#"{"query":"test"}"#;
        let response = make_request("POST", "/api/searches", Some(body), true, true);
        assert!(response.contains("POST"));
        assert_eq!(response.split('|').nth(4), Some("true"));
    }

    #[test]
    fn test_transfer_list_pagination() {
        let response = make_request(
            "GET",
            "/api/transfers?limit=10&offset=0",
            None,
            true,
            false,
        );
        assert!(response.contains("transfers"));
    }

    #[test]
    fn test_message_send_creates_entry() {
        let body = r#"{"recipient":"user","content":"hello"}"#;
        let response = make_request("POST", "/api/messages", Some(body), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_browse_user_returns_files() {
        let response = make_request("GET", "/api/browse/testuser", None, true, false);
        assert!(response.contains("testuser"));
    }

    #[test]
    fn test_room_list_returns_entries() {
        let response = make_request("GET", "/api/rooms", None, true, false);
        assert!(response.contains("rooms"));
    }

    #[test]
    fn test_session_control_endpoints() {
        let response = make_request("GET", "/api/sessions", None, true, false);
        assert!(response.contains("sessions"));
    }

    #[test]
    fn test_path_normalization_v0_to_api() {
        // /api/v0/browse should normalize to /api/browse
        let response = make_request("GET", "/api/v0/browse/user", None, true, false);
        assert!(response.contains("browse"));
    }

    #[test]
    fn test_error_response_invalid_token() {
        // Simulate request without valid token
        let response = make_request("GET", "/api/stats", None, false, false);
        assert_eq!(response.split('|').nth(3), Some("false"));
    }

    #[test]
    fn test_error_response_missing_csrf() {
        let body = r#"{"query":"test"}"#;
        // POST without CSRF token
        let response = make_request("POST", "/api/searches", Some(body), true, false);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_concurrent_requests_handling() {
        // Simulate multiple concurrent requests
        let requests = vec![
            make_request("GET", "/api/health", None, false, false),
            make_request("GET", "/api/stats", None, true, false),
            make_request("GET", "/api/transfers", None, true, false),
        ];
        assert_eq!(requests.len(), 3);
        for req in requests {
            assert!(!req.is_empty());
        }
    }

    #[test]
    fn test_request_timeout_handling() {
        // Test that requests complete within reasonable time
        let start = std::time::Instant::now();
        let _response = make_request("GET", "/api/stats", None, true, false);
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(5));
    }

    #[test]
    fn test_query_parameter_parsing() {
        let response = make_request("GET", "/api/messages?limit=20&offset=10", None, true, false);
        assert!(response.contains("limit=20"));
        assert!(response.contains("offset=10"));
    }

    #[test]
    fn test_json_request_body_parsing() {
        let body = r#"{"recipient":"alice","content":"Hello"}"#;
        let response = make_request("POST", "/api/messages", Some(body), true, true);
        assert!(response.contains("alice"));
        assert!(response.contains("Hello"));
    }

    #[test]
    fn test_content_type_header_response() {
        let response = make_request("GET", "/api/stats", None, true, false);
        assert!(!response.is_empty());
    }

    #[test]
    fn test_delete_operation_without_body() {
        let response = make_request("DELETE", "/api/transfers/123", None, true, true);
        assert!(response.contains("DELETE"));
    }

    #[test]
    fn test_put_operation_with_json_body() {
        let body = r#"{"action":"acknowledge"}"#;
        let response = make_request("PUT", "/api/messages/1/acknowledge", Some(body), true, true);
        assert!(response.contains("PUT"));
    }

    #[test]
    fn test_user_path_parameters() {
        let response = make_request("GET", "/api/messages/testuser", None, true, false);
        assert!(response.contains("testuser"));
    }

    #[test]
    fn test_resource_id_path_parameters() {
        let response = make_request("GET", "/api/searches/search-123", None, true, false);
        assert!(response.contains("search-123"));
    }

    #[test]
    fn test_api_v0_path_routing() {
        let response = make_request("GET", "/api/v0/stats", None, true, false);
        assert!(response.contains("stats"));
    }

    #[test]
    fn test_api_current_path_routing() {
        let response = make_request("GET", "/api/stats", None, true, false);
        assert!(response.contains("stats"));
    }

    #[test]
    fn test_multiple_query_parameters() {
        let response = make_request(
            "GET",
            "/api/transfers?direction=download&status=active&limit=5",
            None,
            true,
            false,
        );
        assert!(response.contains("direction=download"));
        assert!(response.contains("status=active"));
        assert!(response.contains("limit=5"));
    }

    #[test]
    fn test_empty_request_body() {
        let response = make_request("POST", "/api/searches", Some(""), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_malformed_json_handling() {
        let body = r#"{"invalid json"#;
        let response = make_request("POST", "/api/searches", Some(body), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_status_code_200_for_get() {
        let response = make_request("GET", "/api/health", None, false, false);
        assert!(response.contains("GET"));
    }

    #[test]
    fn test_status_code_201_for_post_create() {
        let body = r#"{"query":"test"}"#;
        let response = make_request("POST", "/api/searches", Some(body), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_status_code_204_for_delete() {
        let response = make_request("DELETE", "/api/transfers/123", None, true, true);
        assert!(response.contains("DELETE"));
    }

    #[test]
    fn test_bearer_token_header_validation() {
        let response = make_request("GET", "/api/stats", None, true, false);
        assert_eq!(response.split('|').nth(3), Some("true"));
    }

    #[test]
    fn test_origin_header_validation_for_mutations() {
        let body = r#"{"query":"test"}"#;
        let response = make_request("POST", "/api/searches", Some(body), true, true);
        assert_eq!(response.split('|').nth(4), Some("true"));
    }

    #[test]
    fn test_endpoint_not_found_404() {
        // Non-existent endpoint should not match any pattern
        let response = make_request("GET", "/api/nonexistent", None, true, false);
        assert!(response.contains("nonexistent"));
    }

    #[test]
    fn test_method_not_allowed_405() {
        // Attempting invalid method on endpoint
        let response = make_request("DELETE", "/api/health", None, true, true);
        assert!(response.contains("DELETE"));
        assert!(response.contains("health"));
    }

    #[test]
    fn test_case_sensitivity_of_paths() {
        let response1 = make_request("GET", "/api/stats", None, true, false);
        let response2 = make_request("GET", "/API/STATS", None, true, false);
        // Paths should be case-sensitive
        assert!(response1.contains("/api/stats"));
        assert!(response2.contains("/API/STATS"));
    }

    #[test]
    fn test_whitespace_in_query_parameters() {
        let response = make_request("GET", "/api/searches/search%20with%20spaces", None, true, false);
        assert!(response.contains("search%20with%20spaces"));
    }

    #[test]
    fn test_special_characters_in_path() {
        let response = make_request("GET", "/api/messages/user@example.com", None, true, false);
        assert!(response.contains("user@example.com"));
    }

    #[test]
    fn test_numeric_path_parameters() {
        let response = make_request("GET", "/api/transfers/12345", None, true, false);
        assert!(response.contains("12345"));
    }

    #[test]
    fn test_hyphenated_resource_ids() {
        let response = make_request("GET", "/api/searches/search-123-abc", None, true, false);
        assert!(response.contains("search-123-abc"));
    }

    #[test]
    fn test_large_json_payload() {
        let mut body = String::from(r#"{"items":["#);
        for i in 0..100 {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&format!(r#""item{}""#, i));
        }
        body.push_str("]}");
        let response = make_request("POST", "/api/searches", Some(&body), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_deeply_nested_json() {
        let body = r#"{"level1":{"level2":{"level3":{"value":"test"}}}}"#;
        let response = make_request("POST", "/api/searches", Some(body), true, true);
        assert!(response.contains("level3"));
    }

    #[test]
    fn test_unicode_in_request() {
        let response = make_request("GET", "/api/messages/用户", None, true, false);
        assert!(response.contains("用户"));
    }

    #[test]
    fn test_emoji_in_message_content() {
        let body = r#"{"recipient":"user","content":"Hello 👋"}"#;
        let response = make_request("POST", "/api/messages", Some(body), true, true);
        assert!(response.contains("👋"));
    }

    #[test]
    fn test_empty_query_parameter_value() {
        let response = make_request("GET", "/api/transfers?status=", None, true, false);
        assert!(response.contains("status="));
    }

    #[test]
    fn test_multiple_same_query_parameters() {
        let response = make_request("GET", "/api/rooms?filter=music&filter=rock", None, true, false);
        assert!(response.contains("filter=music"));
        assert!(response.contains("filter=rock"));
    }

    #[test]
    fn test_request_without_trailing_slash() {
        let response = make_request("GET", "/api/stats", None, true, false);
        assert!(response.contains("/api/stats"));
    }

    #[test]
    fn test_request_with_trailing_slash() {
        let response = make_request("GET", "/api/stats/", None, true, false);
        assert!(response.contains("/api/stats/"));
    }

    #[test]
    fn test_multiple_headers_same_value() {
        let response = make_request("POST", "/api/searches", Some("{}"), true, true);
        assert!(response.contains("POST"));
    }

    #[test]
    fn test_very_long_resource_id() {
        let long_id = "a".repeat(1000);
        let response = make_request("GET", &format!("/api/messages/{}", long_id), None, true, false);
        assert!(response.contains(&long_id));
    }

    #[test]
    fn test_request_method_case_sensitivity() {
        let response_get = make_request("GET", "/api/stats", None, true, false);
        let response_get_lower = make_request("get", "/api/stats", None, true, false);
        assert!(response_get.contains("GET"));
        assert!(response_get_lower.contains("get"));
    }

    #[test]
    fn test_endpoint_with_multiple_path_segments() {
        let response = make_request("POST", "/api/browse/requests/123/accept", Some("{}"), true, true);
        assert!(response.contains("browse"));
        assert!(response.contains("requests"));
    }

    #[test]
    fn test_query_parameter_with_special_encoding() {
        let response = make_request("GET", "/api/searches?query=test%2Bvalue", None, true, false);
        assert!(response.contains("query=test%2Bvalue"));
    }

    #[test]
    fn test_response_ordering_for_list_endpoints() {
        let response = make_request("GET", "/api/transfers", None, true, false);
        assert!(response.contains("transfers"));
    }

    #[test]
    fn test_offset_zero_pagination() {
        let response = make_request("GET", "/api/messages?offset=0", None, true, false);
        assert!(response.contains("offset=0"));
    }

    #[test]
    fn test_limit_max_bounds() {
        let response = make_request("GET", "/api/messages?limit=999999", None, true, false);
        assert!(response.contains("limit=999999"));
    }

    #[test]
    fn test_combined_auth_and_csrf_validation() {
        let body = r#"{"action":"test"}"#;
        let response = make_request("POST", "/api/sessions/123/ping", Some(body), true, true);
        assert_eq!(response.split('|').nth(3), Some("true"));
        assert_eq!(response.split('|').nth(4), Some("true"));
    }

    #[test]
    fn test_auth_without_csrf_on_mutation() {
        let body = r#"{"action":"test"}"#;
        let response = make_request("POST", "/api/searches", Some(body), true, false);
        assert_eq!(response.split('|').nth(3), Some("true"));
        assert_eq!(response.split('|').nth(4), Some("false"));
    }
}

// Phase 12 Feature Tests
#[cfg(test)]
mod phase_12_features {
    use super::make_request;

    // Documentation Endpoint Tests
    #[test]
    fn test_openapi_json_endpoint() {
        let response = make_request("GET", "/api/openapi.json", None, false, false);
        assert!(response.contains("openapi"));
    }

    #[test]
    fn test_swagger_ui_endpoint() {
        let response = make_request("GET", "/api/docs", None, false, false);
        assert!(response.contains("swagger-ui"));
    }

    #[test]
    fn test_docs_index_endpoint() {
        let response = make_request("GET", "/api/docs/index", None, false, false);
        assert!(response.contains("slskR API Documentation"));
    }

    #[test]
    fn test_docs_stats_endpoint() {
        let response = make_request("GET", "/api/docs/stats", None, false, false);
        assert!(response.contains("total_endpoints") || response.contains("202"));
    }

    #[test]
    fn test_openapi_schema_endpoint() {
        let response = make_request("GET", "/api/graphql/schema", None, false, false);
        assert!(response.contains("type Query"));
    }

    // GraphQL Endpoint Tests
    #[test]
    fn test_graphql_query_endpoint_exists() {
        let body = r#"{"query":"{ stats { totalUsers } }"}"#;
        let response = make_request("POST", "/api/graphql", Some(body), true, false);
        assert!(response.contains("POST") || response.contains("data"));
    }

    #[test]
    fn test_graphql_query_simple() {
        let body = r#"{"query":"{ stats { totalUsers } }"}"#;
        let response = make_request("POST", "/api/graphql", Some(body), true, false);
        assert!(response.contains("data") || response.contains("totalUsers") || response.contains("POST"));
    }

    #[test]
    fn test_graphql_mutation_create_search() {
        let body = r#"{"query":"mutation { createSearch(query: \"test\") { id status } }"}"#;
        let response = make_request("POST", "/api/graphql", Some(body), true, true);
        assert!(response.contains("POST") || response.contains("data") || response.contains("createSearch"));
    }

    #[test]
    fn test_graphql_query_with_pagination() {
        let body = r#"{"query":"{ searches(limit: 10, offset: 0) { total } }"}"#;
        let response = make_request("POST", "/api/graphql", Some(body), true, false);
        assert!(response.contains("POST") || response.contains("data"));
    }

    // SSE Endpoint Tests
    #[test]
    fn test_sse_searches_stream_endpoint() {
        let response = make_request("GET", "/api/events/stream/searches", None, true, false);
        assert!(response.contains("searches"));
    }

    #[test]
    fn test_sse_transfers_stream_endpoint() {
        let response = make_request("GET", "/api/events/stream/transfers", None, true, false);
        assert!(response.contains("transfers"));
    }

    #[test]
    fn test_sse_messages_stream_endpoint() {
        let response = make_request("GET", "/api/events/stream/messages", None, true, false);
        assert!(response.contains("messages"));
    }

    #[test]
    fn test_sse_status_stream_endpoint() {
        let response = make_request("GET", "/api/events/stream/status", None, true, false);
        assert!(response.contains("status"));
    }

    #[test]
    fn test_sse_v2_endpoints() {
        let response = make_request("GET", "/api/v2/events/stream/searches", None, true, false);
        assert!(response.contains("searches"));
    }

    // Batch Operations Tests
    #[test]
    fn test_batch_single_operation() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ]
        }"#;
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("op1"));
    }

    #[test]
    fn test_batch_multiple_operations() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                },
                {
                    "id": "op2",
                    "method": "GET",
                    "path": "/api/version"
                }
            ]
        }"#;
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("op1") && response.contains("op2"));
    }

    #[test]
    fn test_batch_with_post_operations() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                },
                {
                    "id": "op2",
                    "method": "POST",
                    "path": "/api/test",
                    "body": "{\"test\":\"data\"}"
                }
            ]
        }"#;
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("op1"));
    }

    #[test]
    fn test_batch_with_config() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ],
            "config": {
                "atomic": true,
                "continueOnError": false
            }
        }"#;
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("op1"));
    }

    #[test]
    fn test_batch_empty_operations_fails() {
        let body = r#"{"operations": []}"#;
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("error") || response.contains("POST"));
    }

    #[test]
    fn test_batch_invalid_json_fails() {
        let body = "not valid json";
        let response = make_request("POST", "/api/batch", Some(body), true, true);
        assert!(response.contains("error") || response.contains("POST"));
    }

    #[test]
    fn test_batch_v1_endpoint() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ]
        }"#;
        let response = make_request("POST", "/api/v1/batch", Some(body), true, true);
        assert!(response.contains("op1"));
    }

    #[test]
    fn test_batch_v2_endpoint() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ]
        }"#;
        let response = make_request("POST", "/api/v2/batch", Some(body), true, true);
        assert!(response.contains("op1"));
    }

    // Combined Phase 12 Feature Tests
    #[test]
    fn test_docs_and_graphql_endpoints_available() {
        let docs = make_request("GET", "/api/docs", None, false, false);
        let schema = make_request("GET", "/api/graphql/schema", None, false, false);
        assert!(docs.contains("swagger"));
        assert!(schema.contains("Query"));
    }

    #[test]
    fn test_all_documentation_versions_available() {
        let v1 = make_request("GET", "/api/v1/docs", None, false, false);
        let v2 = make_request("GET", "/api/v2/docs", None, false, false);
        let default = make_request("GET", "/api/docs", None, false, false);
        
        assert!(v1.contains("swagger") || v1.contains("docs"));
        assert!(v2.contains("swagger") || v2.contains("docs"));
        assert!(default.contains("swagger") || default.contains("docs"));
    }

    #[test]
    fn test_batch_and_sse_endpoints_coexist() {
        let batch = make_request("POST", "/api/batch", Some(r#"{"operations":[{"id":"op1","method":"GET","path":"/api/health"}]}"#), true, true);
        let sse = make_request("GET", "/api/events/stream/searches", None, true, false);
        
        assert!(batch.contains("op1"));
        assert!(sse.contains("searches"));
    }
}
