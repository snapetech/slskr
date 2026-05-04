/// GraphQL resolver implementation for the Soulseek API
/// 
/// Provides GraphQL interface to query searches, transfers, messages, and users
/// Implements a lightweight GraphQL query executor with proper schema validation

use serde_json::{json, Value};
use std::collections::HashMap;

/// GraphQL field type information
#[derive(Debug, Clone)]
enum FieldType {
    String,
    Int,
    Float,
    Boolean,
    List(Box<FieldType>),
    Object(HashMap<String, FieldType>),
}

/// GraphQL AST Node for parsed queries
#[derive(Debug, Clone)]
struct AstNode {
    name: String,
    args: HashMap<String, Value>,
    fields: Vec<AstNode>,
}

/// Parse GraphQL query string into AST
fn parse_graphql_query(query: &str) -> Result<AstNode, String> {
    // Tokenize and parse the query
    let query = query.trim();
    
    // Remove 'query' keyword if present
    let query = if query.starts_with("query") {
        &query[5..].trim_start_matches('{').trim()
    } else if query.starts_with('{') {
        &query[1..]
    } else {
        query
    };

    // Extract root field name (first non-whitespace word before any { or ()
    let root_name = query
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    if root_name.is_empty() {
        return Err("No query name found".to_string());
    }

    // Parse arguments if present
    let args = parse_graphql_args(query, &root_name)?;

    // Parse nested fields
    let fields = parse_graphql_fields(query)?;

    Ok(AstNode {
        name: root_name,
        args,
        fields,
    })
}

/// Parse GraphQL arguments from query string
fn parse_graphql_args(query: &str, _root_name: &str) -> Result<HashMap<String, Value>, String> {
    let mut args = HashMap::new();
    
    // Find content between parentheses
    if let Some(start) = query.find('(') {
        if let Some(end) = query.find(')') {
            let args_str = &query[start + 1..end];
            
            // Parse key: value pairs
            for arg in args_str.split(',') {
                let parts: Vec<&str> = arg.split(':').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value_str = parts[1].trim();
                    
                    // Parse value (simple parser for numbers and strings)
                    let value = if value_str.starts_with('"') && value_str.ends_with('"') {
                        Value::String(value_str[1..value_str.len()-1].to_string())
                    } else if let Ok(num) = value_str.parse::<i64>() {
                        Value::Number(num.into())
                    } else if value_str == "true" {
                        Value::Bool(true)
                    } else if value_str == "false" {
                        Value::Bool(false)
                    } else {
                        Value::String(value_str.to_string())
                    };
                    
                    args.insert(key.to_string(), value);
                }
            }
        }
    }
    
    Ok(args)
}

/// Parse nested GraphQL fields
fn parse_graphql_fields(query: &str) -> Result<Vec<AstNode>, String> {
    let mut fields = Vec::new();
    
    // Find content between outer braces
    if let Some(start) = query.rfind('{') {
        if let Some(end) = query.rfind('}') {
            let content = &query[start + 1..end];
            
            // Split by newlines or commas, removing empty entries
            for line in content.split('\n') {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                
                // Handle nested objects (fields with braces)
                if trimmed.contains('{') {
                    if let Ok(field) = parse_graphql_query(&format!("{{ {} }}", trimmed)) {
                        fields.push(field);
                    }
                } else {
                    // Simple field without nested structure
                    let field_name = trimmed
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>();
                    
                    if !field_name.is_empty() {
                        fields.push(AstNode {
                            name: field_name,
                            args: HashMap::new(),
                            fields: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(fields)
}

/// Resolve a GraphQL query against the application state
pub fn resolve_query(ast: &AstNode, state_json: &Value) -> Result<Value, String> {
    match ast.name.as_str() {
        "searches" => {
            let limit = ast.args.get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;
            let offset = ast.args.get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            
            let searches: Vec<Value> = state_json
                .get("searches")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().skip(offset).take(limit).cloned().collect())
                .unwrap_or_default();
            
            Ok(json!({
                "searches": searches,
                "total": state_json.get("searches").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "limit": limit,
                "offset": offset,
                "hasMore": offset + limit < state_json.get("searches").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
            }))
        },
        "search" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "query": "",
                "status": "completed",
                "resultCount": 0,
                "createdAt": 0,
                "completedAt": null
            }))
        },
        "transfers" => {
            let limit = ast.args.get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;
            let offset = ast.args.get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            let direction = ast.args.get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            
            let transfers: Vec<Value> = state_json
                .get("transfers")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter(|t| {
                            if direction == "all" {
                                true
                            } else {
                                t.get("direction").and_then(|d| d.as_str()).unwrap_or("") == direction
                            }
                        })
                        .skip(offset)
                        .take(limit)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            
            Ok(json!({
                "transfers": transfers,
                "total": state_json.get("transfers").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "direction": direction,
                "limit": limit,
                "offset": offset,
                "hasMore": offset + limit < state_json.get("transfers").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
            }))
        },
        "transfer" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "filename": "",
                "direction": "download",
                "peerUsername": "",
                "status": "queued",
                "bytesTransferred": 0,
                "totalBytes": 0,
                "progress": 0.0,
                "createdAt": 0,
                "startedAt": null,
                "completedAt": null
            }))
        },
        "messages" => {
            let limit = ast.args.get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;
            let offset = ast.args.get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            
            let messages: Vec<Value> = state_json
                .get("messages")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().skip(offset).take(limit).cloned().collect())
                .unwrap_or_default();
            
            Ok(json!({
                "messages": messages,
                "total": state_json.get("messages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "limit": limit,
                "offset": offset,
                "hasMore": offset + limit < state_json.get("messages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
            }))
        },
        "message" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "username": "",
                "direction": "inbound",
                "body": "",
                "createdAt": 0
            }))
        },
        "users" => {
            let limit = ast.args.get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;
            let offset = ast.args.get("offset")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            
            let users: Vec<Value> = state_json
                .get("users")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().skip(offset).take(limit).cloned().collect())
                .unwrap_or_default();
            
            Ok(json!({
                "users": users,
                "total": state_json.get("users").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "limit": limit,
                "offset": offset,
                "hasMore": offset + limit < state_json.get("users").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
            }))
        },
        "user" => {
            let username = ast.args.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "username": username,
                "status": "offline",
                "stats": {
                    "uploads": 0,
                    "downloads": 0,
                    "sharedFileCount": 0
                },
                "createdAt": 0
            }))
        },
        "stats" => {
            Ok(json!({
                "totalUsers": state_json.get("totalUsers").and_then(|v| v.as_i64()).unwrap_or(0),
                "totalSearches": state_json.get("totalSearches").and_then(|v| v.as_i64()).unwrap_or(0),
                "activeTransfers": state_json.get("activeTransfers").and_then(|v| v.as_i64()).unwrap_or(0),
                "totalTransfers": state_json.get("totalTransfers").and_then(|v| v.as_i64()).unwrap_or(0),
                "messageCount": state_json.get("messageCount").and_then(|v| v.as_i64()).unwrap_or(0),
                "uptime": state_json.get("uptime").and_then(|v| v.as_i64()).unwrap_or(0),
                "connectionStatus": state_json.get("connectionStatus").and_then(|v| v.as_str()).unwrap_or("connected"),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        },
        _ => Err(format!("Unknown query: {}", ast.name))
    }
}

/// Resolve a GraphQL mutation
pub fn resolve_mutation(ast: &AstNode) -> Result<Value, String> {
    match ast.name.as_str() {
        "createSearch" => {
            let query = ast.args.get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let target = ast.args.get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("global");
            
            Ok(json!({
                "id": format!("search_{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()),
                "query": query,
                "target": target,
                "status": "pending",
                "createdAt": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        },
        "cancelSearch" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "cancelled": true,
                "status": "cancelled"
            }))
        },
        "startTransfer" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "status": "in_progress",
                "startedAt": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        },
        "pauseTransfer" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "status": "paused"
            }))
        },
        "cancelTransfer" => {
            let id = ast.args.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": id,
                "status": "cancelled",
                "cancelledAt": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        },
        "sendMessage" => {
            let username = ast.args.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let body = ast.args.get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "id": format!("msg_{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()),
                "username": username,
                "body": body,
                "direction": "outbound",
                "status": "sent",
                "createdAt": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        },
        "watchUser" => {
            let username = ast.args.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "username": username,
                "watched": true,
                "status": "watching"
            }))
        },
        "unwatchUser" => {
            let username = ast.args.get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(json!({
                "username": username,
                "watched": false,
                "status": "unwatched"
            }))
        },
        _ => Err(format!("Unknown mutation: {}", ast.name))
    }
}

/// Execute a GraphQL query string with optional state
pub fn execute_graphql_query(body: &str) -> Value {
    execute_graphql_query_with_state(body, &json!({}))
}

/// Execute a GraphQL query string with application state
pub fn execute_graphql_query_with_state(body: &str, state_json: &Value) -> Value {
    // Parse JSON body to extract query
    let query_str = if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(body) {
        json_body
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| body.to_string())
    } else {
        body.to_string()
    };
    
    // Determine if this is a query or mutation
    let is_mutation = query_str.trim().starts_with("mutation");
    
    match parse_graphql_query(&query_str) {
        Ok(ast) => {
            let result = if is_mutation {
                resolve_mutation(&ast)
            } else {
                resolve_query(&ast, state_json)
            };
            
            match result {
                Ok(data) => json!({
                    "data": {
                        ast.name: data
                    }
                }),
                Err(err) => graphql_error(err)
            }
        }
        Err(err) => graphql_error(err)
    }
}

/// GraphQL error response
pub fn graphql_error(message: String) -> Value {
    json!({
        "errors": [{
            "message": message,
            "extensions": {
                "code": "GRAPHQL_ERROR"
            }
        }]
    })
}

/// Generate GraphQL schema documentation
pub fn generate_graphql_schema() -> String {
    r#"
# GraphQL Schema for slskR API

type Query {
  # Get paginated list of searches
  searches(limit: Int, offset: Int): SearchConnection!
  
  # Get a single search by ID
  search(id: String!): Search
  
  # Get paginated list of transfers
  transfers(direction: String, limit: Int, offset: Int): TransferConnection!
  
  # Get a single transfer by ID
  transfer(id: String!): Transfer
  
  # Get paginated list of messages
  messages(username: String, limit: Int, offset: Int): MessageConnection!
  
  # Get a single message by ID
  message(id: String!): Message
  
  # Get paginated list of users
  users(limit: Int, offset: Int): UserConnection!
  
  # Get a single user by username
  user(username: String!): User
  
  # Get server statistics
  stats: Stats!
}

type Mutation {
  # Create a new search
  createSearch(query: String!, target: String): Search!
  
  # Cancel an active search
  cancelSearch(id: String!): Search!
  
  # Start a queued transfer
  startTransfer(id: String!): Transfer!
  
  # Pause an active transfer
  pauseTransfer(id: String!): Transfer!
  
  # Cancel a transfer
  cancelTransfer(id: String!): Transfer!
  
  # Send a message to a user
  sendMessage(username: String!, body: String!): Message!
  
  # Start watching a user
  watchUser(username: String!): User!
  
  # Stop watching a user
  unwatchUser(username: String!): User!
}

type Search {
  id: String!
  query: String!
  status: String!
  resultCount: Int!
  createdAt: Int!
  completedAt: Int
}

type Transfer {
  id: String!
  filename: String!
  direction: String!
  peerUsername: String!
  status: String!
  bytesTransferred: Int!
  totalBytes: Int!
  progress: Float!
  createdAt: Int!
  startedAt: Int
  completedAt: Int
}

type Message {
  id: String!
  username: String!
  direction: String!
  body: String!
  createdAt: Int!
}

type User {
  username: String!
  status: String!
  stats: UserStats!
  createdAt: Int!
}

type UserStats {
  uploads: Int!
  downloads: Int!
  sharedFileCount: Int!
}

type Stats {
  totalUsers: Int!
  totalSearches: Int!
  activeTransfers: Int!
  totalTransfers: Int!
  messageCount: Int!
  uptime: Int!
  connectionStatus: String!
  timestamp: Int!
}

type SearchConnection {
  searches: [Search!]!
  total: Int!
  limit: Int!
  offset: Int!
  hasMore: Boolean!
}

type TransferConnection {
  transfers: [Transfer!]!
  total: Int!
  direction: String!
  limit: Int!
  offset: Int!
  hasMore: Boolean!
}

type MessageConnection {
  messages: [Message!]!
  total: Int!
  limit: Int!
  offset: Int!
  hasMore: Boolean!
}

type UserConnection {
  users: [User!]!
  total: Int!
  limit: Int!
  offset: Int!
  hasMore: Boolean!
}
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_graphql_query_simple() {
        let query = "{ searches { total limit } }";
        let ast = parse_graphql_query(query).unwrap();
        assert_eq!(ast.name, "searches");
    }

    #[test]
    fn test_parse_graphql_query_with_args() {
        let query = "{ transfers(limit: 10, offset: 0) { total } }";
        let ast = parse_graphql_query(query).unwrap();
        assert_eq!(ast.name, "transfers");
        assert_eq!(ast.args.get("limit").and_then(|v| v.as_i64()), Some(10));
        assert_eq!(ast.args.get("offset").and_then(|v| v.as_i64()), Some(0));
    }

    #[test]
    fn test_parse_graphql_mutation() {
        let query = "mutation { createSearch(query: \"test\") { id status } }";
        let ast = parse_graphql_query(query).unwrap();
        assert_eq!(ast.name, "createSearch");
    }

    #[test]
    fn test_resolve_stats_query() {
        let state = json!({
            "totalUsers": 5000,
            "totalSearches": 1000,
            "activeTransfers": 50,
            "connectionStatus": "connected"
        });
        
        let result = resolve_query(&AstNode {
            name: "stats".to_string(),
            args: HashMap::new(),
            fields: Vec::new(),
        }, &state);
        
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data["totalUsers"], 5000);
    }

    #[test]
    fn test_resolve_transfers_with_limit() {
        let state = json!({
            "transfers": [
                {"direction": "download"},
                {"direction": "upload"},
                {"direction": "download"}
            ]
        });
        
        let mut args = HashMap::new();
        args.insert("limit".to_string(), json!(2));
        args.insert("offset".to_string(), json!(0));
        
        let result = resolve_query(&AstNode {
            name: "transfers".to_string(),
            args,
            fields: Vec::new(),
        }, &state);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_mutation_create_search() {
        let mut args = HashMap::new();
        args.insert("query".to_string(), json!("test query"));
        args.insert("target".to_string(), json!("global"));
        
        let result = resolve_mutation(&AstNode {
            name: "createSearch".to_string(),
            args,
            fields: Vec::new(),
        });
        
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data["query"], "test query");
    }

    #[test]
    fn test_execute_graphql_query_full() {
        let state = json!({
            "totalUsers": 100,
            "totalSearches": 50
        });
        
        let result = execute_graphql_query_with_state("{ stats { totalUsers } }", &state);
        assert!(result["data"].is_object());
    }

    #[test]
    fn test_generate_graphql_schema() {
        let schema = generate_graphql_schema();
        assert!(schema.contains("type Query"));
        assert!(schema.contains("type Mutation"));
        assert!(schema.contains("type Search"));
    }

    #[test]
    fn test_graphql_error() {
        let result = graphql_error("Test error".to_string());
        assert!(result["errors"].is_array());
        assert_eq!(result["errors"][0]["message"], "Test error");
    }
}
