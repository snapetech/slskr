use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::StatusCode;

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn daemon_http_api_smoke() {
    let port = unused_loopback_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-api-smoke-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&state_dir).unwrap();

    let binary = option_env!("CARGO_BIN_EXE_slskr")
        .map(str::to_owned)
        .unwrap_or_else(|| std::env::var("CARGO_BIN_EXE_slskr").expect("slskr binary path"));
    let child = Command::new(binary)
        .arg("serve")
        .env("SLSKR_HTTP_BIND", format!("127.0.0.1:{port}"))
        .env("SLSKR_STATE_DIR", &state_dir)
        .env("SLSKR_API_TOKEN", "smoke-token")
        .env("SLSKR_API_RATE_LIMIT_AUTHENTICATED", "4")
        .env("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn slskr serve");
    let _guard = ChildGuard { child };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    wait_for_health(&client, &base_url).await;

    let health = client
        .get(format!("{base_url}/api/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    let version = client
        .get(format!("{base_url}/api/version"))
        .send()
        .await
        .unwrap();
    assert_eq!(version.status(), StatusCode::OK);

    let capabilities = client
        .get(format!("{base_url}/api/capabilities"))
        .send()
        .await
        .unwrap();
    assert_eq!(capabilities.status(), StatusCode::OK);

    let missing_auth = client
        .get(format!("{base_url}/api/v0/config"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_auth.status(), StatusCode::UNAUTHORIZED);

    let bad_csrf = client
        .post(format!("{base_url}/api/v0/searches"))
        .bearer_auth("smoke-token")
        .header("Origin", "http://evil.invalid")
        .json(&serde_json::json!({"query": "test flac"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_csrf.status(), StatusCode::FORBIDDEN);

    let created = client
        .post(format!("{base_url}/api/v0/searches"))
        .bearer_auth("smoke-token")
        .json(&serde_json::json!({"query": "test flac"}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = created.text().await.unwrap();
    assert!(created_body.contains("\"query\":\"test flac\""));

    let searches = client
        .get(format!("{base_url}/api/v0/searches"))
        .bearer_auth("smoke-token")
        .send()
        .await
        .unwrap();
    assert_eq!(searches.status(), StatusCode::OK);
    let searches_body = searches.text().await.unwrap();
    assert!(searches_body.contains("\"count\":1"));

    let transfers = client
        .get(format!("{base_url}/api/v0/transfers"))
        .bearer_auth("smoke-token")
        .send()
        .await
        .unwrap();
    assert_eq!(transfers.status(), StatusCode::OK);

    let rate_limited = client
        .get(format!("{base_url}/api/v0/config"))
        .bearer_auth("smoke-token")
        .send()
        .await
        .unwrap();
    assert_eq!(rate_limited.status(), StatusCode::TOO_MANY_REQUESTS);
}

async fn wait_for_health(client: &reqwest::Client, base_url: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(response) = client.get(format!("{base_url}/api/health")).send().await {
            if response.status() == StatusCode::OK {
                return;
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "slskr serve did not become healthy"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn unused_loopback_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
