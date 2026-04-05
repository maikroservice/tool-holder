use std::io::Write;

use tempfile::NamedTempFile;
use tool_holder::config::{FileConfig, FileFormat};
use tool_holder::connector::Connector;
use tool_holder::connector::file::FileConnector;

fn temp_file_with(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

// ── JSON ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_json_array_returns_rows() {
    let f = temp_file_with(r#"[{"token":"abc","type":"bearer"},{"token":"xyz","type":"basic"}]"#);
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Json,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["token"], "abc");
    assert_eq!(rows[1]["token"], "xyz");
}

#[tokio::test]
async fn test_json_single_object_returns_one_row() {
    let f = temp_file_with(r#"{"token":"abc","type":"bearer"}"#);
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Json,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["token"], "abc");
}

#[tokio::test]
async fn test_json_invalid_content_returns_error() {
    let f = temp_file_with("not json at all");
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Json,
        path: f.path().to_str().unwrap().to_string(),
    });

    assert!(connector.fetch().await.is_err());
}

// ── YAML ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_yaml_array_returns_rows() {
    let f = temp_file_with("- token: abc\n  type: bearer\n- token: xyz\n  type: basic\n");
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Yaml,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["token"], "abc");
}

#[tokio::test]
async fn test_yaml_single_object_returns_one_row() {
    let f = temp_file_with("token: abc\ntype: bearer\n");
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Yaml,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["token"], "abc");
}

// ── TXT ────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_txt_returns_one_row_per_line() {
    let f = temp_file_with("token_one\ntoken_two\ntoken_three\n");
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Txt,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["line"], "token_one");
    assert_eq!(rows[2]["line"], "token_three");
}

#[tokio::test]
async fn test_txt_skips_empty_lines() {
    let f = temp_file_with("token_one\n\ntoken_two\n\n");
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Txt,
        path: f.path().to_str().unwrap().to_string(),
    });

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 2);
}

// ── File not found ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_missing_file_returns_error() {
    let connector = FileConnector::new(FileConfig {
        format: FileFormat::Json,
        path: "/nonexistent/path/results.json".to_string(),
    });

    assert!(connector.fetch().await.is_err());
}
