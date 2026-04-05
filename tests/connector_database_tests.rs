use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use tool_holder::config::{DatabaseConfig, DatabaseDriver};
use tool_holder::connector::Connector;
use tool_holder::connector::database::DatabaseConnector;

fn sqlite_config(db_path: &str, query: &str) -> DatabaseConfig {
    DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        host: db_path.to_string(),
        port: 0,
        database: "main".to_string(),
        table: None,
        columns: None,
        query: Some(query.to_string()),
        cursor_field: None,
        credentials: None,
    }
}

/// Seed a SQLite file using SqlitePool (supports create_if_missing).
/// Statements are split on ';' and executed one at a time.
async fn seed_db(db_path: &str, sql: &str) {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await.unwrap();
    for stmt in sql.split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
    }
    pool.close().await;
}

// ── fetch rows ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_database_connector_returns_rows() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let path_str = db_path.to_str().unwrap();

    seed_db(
        path_str,
        "CREATE TABLE credentials (id INTEGER PRIMARY KEY, token TEXT, kind TEXT);
         INSERT INTO credentials (token, kind) VALUES ('abc', 'bearer');
         INSERT INTO credentials (token, kind) VALUES ('xyz', 'basic');",
    )
    .await;

    let connector = DatabaseConnector::new(sqlite_config(
        path_str,
        "SELECT token, kind FROM credentials ORDER BY id",
    ))
    .await
    .unwrap();

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["token"], "abc");
    assert_eq!(rows[1]["token"], "xyz");
}

#[tokio::test]
async fn test_database_connector_empty_table_returns_empty_vec() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("empty.db");
    let path_str = db_path.to_str().unwrap();

    seed_db(
        path_str,
        "CREATE TABLE credentials (id INTEGER PRIMARY KEY, token TEXT);",
    )
    .await;

    let connector = DatabaseConnector::new(sqlite_config(
        path_str,
        "SELECT token FROM credentials",
    ))
    .await
    .unwrap();

    let rows = connector.fetch().await.unwrap();
    assert!(rows.is_empty());
}

// ── cursor / watermark ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_cursor_filter_skips_already_seen_rows() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("cursor.db");
    let path_str = db_path.to_str().unwrap();

    seed_db(
        path_str,
        "CREATE TABLE credentials (id INTEGER PRIMARY KEY, token TEXT);
         INSERT INTO credentials (token) VALUES ('row1');
         INSERT INTO credentials (token) VALUES ('row2');
         INSERT INTO credentials (token) VALUES ('row3');",
    )
    .await;

    // Simulate cursor at id=1 — only rows with id > 1 should come back.
    let connector = DatabaseConnector::new(sqlite_config(
        path_str,
        "SELECT id, token FROM credentials WHERE id > 1 ORDER BY id",
    ))
    .await
    .unwrap();

    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["token"], "row2");
    assert_eq!(rows[1]["token"], "row3");
}

// ── table + columns config ────────────────────────────────────────────────────

#[tokio::test]
async fn test_table_and_columns_config_builds_select() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("table.db");
    let path_str = db_path.to_str().unwrap();

    seed_db(
        path_str,
        "CREATE TABLE creds (id INTEGER PRIMARY KEY, token TEXT, ignored TEXT);
         INSERT INTO creds (token, ignored) VALUES ('tok1', 'x');",
    )
    .await;

    sqlx::any::install_default_drivers();
    let config = DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        host: path_str.to_string(),
        port: 0,
        database: "main".to_string(),
        table: Some("creds".to_string()),
        columns: Some(vec!["token".to_string()]),
        query: None,
        cursor_field: None,
        credentials: None,
    };

    let connector = DatabaseConnector::new(config).await.unwrap();
    let rows = connector.fetch().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["token"], "tok1");
}
