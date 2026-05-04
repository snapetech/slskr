/// Administrative CLI tool for soulseekR
///
/// Provides command-line management of the HTTP API server,
/// including key management, configuration, monitoring, and maintenance.

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[derive(Parser)]
#[command(name = "soulseekr-admin")]
#[command(about = "soulseekR Administrative CLI Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long, default_value = "http://localhost:8080")]
    api_url: String,

    #[arg(global = true, long)]
    api_key: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// API Key management
    ApiKey(ApiKeyCommand),

    /// Server management
    Server(ServerCommand),

    /// Webhook management
    Webhook(WebhookCommand),

    /// Database operations
    Database(DatabaseCommand),

    /// Monitoring and health checks
    Health(HealthCommand),

    /// Configuration management
    Config(ConfigCommand),
}

#[derive(Parser)]
struct ApiKeyCommand {
    #[command(subcommand)]
    action: ApiKeyAction,
}

#[derive(Subcommand)]
enum ApiKeyAction {
    /// Create new API key
    Create {
        #[arg(long)]
        scopes: Vec<String>,

        #[arg(long)]
        expires_days: Option<u32>,
    },

    /// List all API keys
    List {
        #[arg(long, default_value = "50")]
        limit: u32,

        #[arg(long, default_value = "0")]
        offset: u32,
    },

    /// Get API key details
    Get {
        id: String,
    },

    /// Revoke API key
    Revoke {
        id: String,

        #[arg(long)]
        force: bool,
    },

    /// Rotate API key
    Rotate {
        id: String,
    },
}

#[derive(Parser)]
struct ServerCommand {
    #[command(subcommand)]
    action: ServerAction,
}

#[derive(Subcommand)]
enum ServerAction {
    /// Check server health
    Health,

    /// Get server version
    Version,

    /// Get server statistics
    Stats,

    /// Get server configuration
    Config,

    /// Restart server
    Restart,

    /// Shutdown server
    Shutdown,
}

#[derive(Parser)]
struct WebhookCommand {
    #[command(subcommand)]
    action: WebhookAction,
}

#[derive(Subcommand)]
enum WebhookAction {
    /// Create new webhook
    Create {
        url: String,

        #[arg(long)]
        events: Vec<String>,

        #[arg(long)]
        secret: Option<String>,
    },

    /// List webhooks
    List,

    /// Get webhook details
    Get {
        id: String,
    },

    /// Delete webhook
    Delete {
        id: String,
    },

    /// Test webhook
    Test {
        id: String,
    },
}

#[derive(Parser)]
struct DatabaseCommand {
    #[command(subcommand)]
    action: DatabaseAction,
}

#[derive(Subcommand)]
enum DatabaseAction {
    /// Get database statistics
    Stats,

    /// Cleanup old records
    Cleanup {
        #[arg(long, default_value = "30")]
        days: u32,
    },

    /// Vacuum database
    Vacuum,

    /// Export data
    Export {
        #[arg(long)]
        format: Option<String>,
    },
}

#[derive(Parser)]
struct HealthCommand {
    #[command(subcommand)]
    action: Option<HealthAction>,
}

#[derive(Subcommand)]
enum HealthAction {
    /// Detailed health check
    Check,

    /// Monitor health in real-time
    Monitor {
        #[arg(long, default_value = "5")]
        interval_seconds: u64,
    },
}

#[derive(Parser)]
struct ConfigCommand {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get current configuration
    Get,

    /// Update configuration
    Set {
        key: String,
        value: String,
    },

    /// Validate configuration
    Validate,

    /// Export configuration
    Export {
        output: String,
    },
}

/// CLI Client for communicating with API
struct CliClient {
    api_url: String,
    api_key: Option<String>,
    client: Client,
}

impl CliClient {
    fn new(api_url: String, api_key: Option<String>) -> Self {
        CliClient {
            api_url,
            api_key,
            client: Client::new(),
        }
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
        let url = format!("{}{}", self.api_url, path);
        let mut req = self.client.get(&url);

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await?;
        Ok(response.json().await?)
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, Box<dyn Error>> {
        let url = format!("{}{}", self.api_url, path);
        let mut req = self.client.post(&url).json(&body);

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await?;
        Ok(response.json().await?)
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
        let url = format!("{}{}", self.api_url, path);
        let mut req = self.client.delete(&url);

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await?;
        Ok(response.json().await?)
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let client = CliClient::new(cli.api_url, cli.api_key);

    match cli.command {
        Commands::ApiKey(cmd) => handle_api_key(&client, cmd).await?,
        Commands::Server(cmd) => handle_server(&client, cmd).await?,
        Commands::Webhook(cmd) => handle_webhook(&client, cmd).await?,
        Commands::Database(cmd) => handle_database(&client, cmd).await?,
        Commands::Health(cmd) => handle_health(&client, cmd).await?,
        Commands::Config(cmd) => handle_config(&client, cmd).await?,
    }

    Ok(())
}

async fn handle_api_key(
    client: &CliClient,
    cmd: ApiKeyCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        ApiKeyAction::Create { scopes, expires_days } => {
            let body = json!({
                "scopes": scopes,
                "expires_days": expires_days
            });
            let response = client.post("/api/admin/api-keys", body).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ApiKeyAction::List { limit, offset } => {
            let path = format!("/api/admin/api-keys?limit={}&offset={}", limit, offset);
            let response = client.get(&path).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ApiKeyAction::Get { id } => {
            let response = client.get(&format!("/api/admin/api-keys/{}", id)).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ApiKeyAction::Revoke { id, force: _ } => {
            client.delete(&format!("/api/admin/api-keys/{}", id)).await?;
            println!("API key {} revoked successfully", id);
        }

        ApiKeyAction::Rotate { id } => {
            let response =
                client.post(&format!("/api/admin/api-keys/{}/rotate", id), json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

async fn handle_server(
    client: &CliClient,
    cmd: ServerCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        ServerAction::Health => {
            let response = client.get("/api/health").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ServerAction::Version => {
            let response = client.get("/api/version").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ServerAction::Stats => {
            let response = client.get("/api/stats").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ServerAction::Config => {
            let response = client.get("/api/config").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ServerAction::Restart => {
            client.post("/api/admin/server/restart", json!({})).await?;
            println!("Server restart initiated");
        }

        ServerAction::Shutdown => {
            client.post("/api/admin/server/shutdown", json!({})).await?;
            println!("Server shutdown initiated");
        }
    }

    Ok(())
}

async fn handle_webhook(
    client: &CliClient,
    cmd: WebhookCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        WebhookAction::Create { url, events, secret } => {
            let body = json!({
                "url": url,
                "events": events,
                "secret": secret
            });
            let response = client.post("/api/admin/webhooks", body).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        WebhookAction::List => {
            let response = client.get("/api/admin/webhooks").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        WebhookAction::Get { id } => {
            let response = client.get(&format!("/api/admin/webhooks/{}", id)).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        WebhookAction::Delete { id } => {
            client.delete(&format!("/api/admin/webhooks/{}", id)).await?;
            println!("Webhook {} deleted", id);
        }

        WebhookAction::Test { id } => {
            client.post(&format!("/api/admin/webhooks/{}/test", id), json!({})).await?;
            println!("Test webhook sent to {}", id);
        }
    }

    Ok(())
}

async fn handle_database(
    client: &CliClient,
    cmd: DatabaseCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        DatabaseAction::Stats => {
            let response = client.get("/api/admin/database/stats").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        DatabaseAction::Cleanup { days } => {
            let body = json!({ "days": days });
            let response = client.post("/api/admin/database/cleanup", body).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        DatabaseAction::Vacuum => {
            client.post("/api/admin/database/vacuum", json!({})).await?;
            println!("Database vacuum completed");
        }

        DatabaseAction::Export { format } => {
            let path = match format {
                Some(fmt) => format!("/api/admin/database/export?format={}", fmt),
                None => "/api/admin/database/export".to_string(),
            };
            let response = client.get(&path).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

async fn handle_health(
    client: &CliClient,
    cmd: HealthCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        Some(HealthAction::Check) => {
            let response = client.get("/api/admin/health/check").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        Some(HealthAction::Monitor { interval_seconds }) => {
            loop {
                let response = client.get("/api/health").await?;
                println!("{}: {}", chrono::Local::now(), serde_json::to_string_pretty(&response)?);
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_seconds)).await;
            }
        }

        None => {
            let response = client.get("/api/health").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

async fn handle_config(
    client: &CliClient,
    cmd: ConfigCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.action {
        ConfigAction::Get => {
            let response = client.get("/api/config").await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ConfigAction::Set { key, value } => {
            let body = json!({ key: value });
            client.post("/api/admin/config", body).await?;
            println!("Configuration updated: {} = {}", key, value);
        }

        ConfigAction::Validate => {
            let response = client.post("/api/admin/config/validate", json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        ConfigAction::Export { output } => {
            let response = client.get("/api/config").await?;
            std::fs::write(&output, serde_json::to_string_pretty(&response)?)?;
            println!("Configuration exported to {}", output);
        }
    }

    Ok(())
}
