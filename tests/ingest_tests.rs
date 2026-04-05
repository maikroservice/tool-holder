use std::collections::HashMap;

use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use tool_holder::ingest::AtcClient;

async fn make_client(url: &str) -> AtcClient {
    AtcClient::new(url.to_string(), "atc_test_key".to_string()).unwrap()
}

// ── Happy path ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ingest_sends_correct_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest"))
        .and(header("x-ingest-key", "atc_test_key"))
        .and(body_json(json!({
            "source": "nophish",
            "credentials": { "token": "abc123", "type": "bearer" }
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = make_client(&server.uri()).await;
    let mut creds = HashMap::new();
    creds.insert("token".to_string(), Value::String("abc123".to_string()));
    creds.insert("type".to_string(), Value::String("bearer".to_string()));

    client.ingest("nophish", creds).await.unwrap();
    server.verify().await;
}

#[tokio::test]
async fn test_ingest_includes_ingest_key_header() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest"))
        .and(header("x-ingest-key", "atc_test_key"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = make_client(&server.uri()).await;
    client.ingest("tool", HashMap::new()).await.unwrap();
    server.verify().await;
}

// ── Error handling ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ingest_403_returns_unauthorized_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = make_client(&server.uri()).await;
    let err = client.ingest("tool", HashMap::new()).await.unwrap_err();
    assert!(matches!(err, tool_holder::error::IngestError::Unauthorized));
}

#[tokio::test]
async fn test_ingest_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = make_client(&server.uri()).await;
    let err = client.ingest("tool", HashMap::new()).await.unwrap_err();
    assert!(matches!(
        err,
        tool_holder::error::IngestError::Server { status: 500 }
    ));
}

// ── Mapping integration ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_mapped_row_sent_as_credentials() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/ingest"))
        .and(body_json(json!({
            "source": "nophish",
            "credentials": { "token": "tok_xyz" }
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    // Simulate the mapping step: tool row → ATC fields.
    let mut raw_row = HashMap::new();
    raw_row.insert("credential_value".to_string(), Value::String("tok_xyz".to_string()));
    raw_row.insert("irrelevant_field".to_string(), Value::String("ignored".to_string()));

    let mut mapping = HashMap::new();
    mapping.insert("token".to_string(), "credential_value".to_string());

    let mapped = tool_holder::mapping::apply_mapping(&raw_row, &mapping);

    let client = make_client(&server.uri()).await;
    client.ingest("nophish", mapped).await.unwrap();
    server.verify().await;
}
