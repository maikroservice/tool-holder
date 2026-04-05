use std::collections::HashMap;

use tempfile::TempDir;
use tool_holder::config::{
    AtcConfig, Credentials, DatabaseDriver, FileFormat, MongoConfig, SourceConfig,
    load_all, load_tool_config_from_dir, substitute_env_vars,
};

// ── helpers ────────────────────────────────────────────────────────────────────

/// Write a tool directory with config.yaml and an optional .env file.
fn make_tool_dir(yaml: &str, env: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("config.yaml"), yaml).unwrap();
    if let Some(env_content) = env {
        std::fs::write(dir.path().join(".env"), env_content).unwrap();
    }
    dir
}

// ── substitute_env_vars ────────────────────────────────────────────────────────

#[test]
fn test_substitute_from_local_env() {
    let mut local = HashMap::new();
    local.insert("DB_USER".to_string(), "alice".to_string());
    let result = substitute_env_vars("user: ${DB_USER}", &local).unwrap();
    assert_eq!(result, "user: alice");
}

#[test]
fn test_local_env_takes_precedence_over_global() {
    unsafe { std::env::set_var("TH_PRECEDENCE", "global") };
    let mut local = HashMap::new();
    local.insert("TH_PRECEDENCE".to_string(), "local".to_string());
    let result = substitute_env_vars("val: ${TH_PRECEDENCE}", &local).unwrap();
    assert_eq!(result, "val: local");
}

#[test]
fn test_falls_back_to_global_env() {
    unsafe { std::env::set_var("TH_GLOBAL_ONLY", "from_global") };
    let result = substitute_env_vars("val: ${TH_GLOBAL_ONLY}", &HashMap::new()).unwrap();
    assert_eq!(result, "val: from_global");
}

#[test]
fn test_missing_var_returns_error_with_name() {
    unsafe { std::env::remove_var("TH_DEFINITELY_MISSING") };
    let err = substitute_env_vars("val: ${TH_DEFINITELY_MISSING}", &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("TH_DEFINITELY_MISSING"));
}

#[test]
fn test_no_vars_unchanged() {
    let input = "name: nophish";
    assert_eq!(substitute_env_vars(input, &HashMap::new()).unwrap(), input);
}

// ── load_tool_config_from_dir — database ─���────────────────────────────────────

#[test]
fn test_load_postgres_config_with_local_env() {
    let yaml = r#"
name: nophish
source:
  type: database
  driver: postgres
  host: localhost
  port: 5432
  database: nophish
  table: found_credentials
  columns:
    - credential_value
    - credential_type
  cursor_field: id
  credentials:
    username: ${DB_USER}
    password: ${DB_PASS}
mapping:
  token: credential_value
  type: credential_type
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(
        yaml,
        Some("DB_USER=dbuser\nDB_PASS=dbpass\nINGEST_KEY=atc_abc123"),
    );
    let config = load_tool_config_from_dir(dir.path()).unwrap();

    assert_eq!(config.name, "nophish");
    assert_eq!(
        config.atc,
        AtcConfig {
            url: "http://atc.internal".to_string(),
            ingest_key: "atc_abc123".to_string(),
        }
    );

    let SourceConfig::Database(ref db) = config.source else {
        panic!("expected Database source");
    };
    assert!(matches!(db.driver, DatabaseDriver::Postgres));
    assert_eq!(db.table.as_deref(), Some("found_credentials"));
    assert_eq!(
        db.credentials,
        Some(Credentials {
            username: Some("dbuser".to_string()),
            password: Some("dbpass".to_string()),
        })
    );
}

#[test]
fn test_postgres_connection_url_with_credentials() {
    let yaml = r#"
name: tool
source:
  type: database
  driver: postgres
  host: db.internal
  port: 5432
  database: mydb
  query: "SELECT token FROM creds"
  credentials:
    username: ${DB_USER}
    password: ${DB_PASS}
mapping:
  token: token
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("DB_USER=user\nDB_PASS=pass\nINGEST_KEY=atc_xyz"));
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    let SourceConfig::Database(ref db) = config.source else { panic!() };
    assert_eq!(db.connection_url(), "postgres://user:pass@db.internal:5432/mydb");
}

#[test]
fn test_sqlite_connection_url() {
    let yaml = r#"
name: sqlite_tool
source:
  type: database
  driver: sqlite
  host: /data/nophish.db
  port: 0
  database: main
  table: results
  columns:
    - token
mapping:
  token: token
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("INGEST_KEY=atc_xyz"));
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    let SourceConfig::Database(ref db) = config.source else { panic!() };
    assert_eq!(db.connection_url(), "sqlite:///data/nophish.db");
}

#[test]
fn test_database_config_missing_table_and_query_fails() {
    let yaml = r#"
name: broken_tool
source:
  type: database
  driver: postgres
  host: localhost
  port: 5432
  database: mydb
mapping:
  token: token
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("INGEST_KEY=atc_xyz"));
    assert!(load_tool_config_from_dir(dir.path()).is_err());
}

#[test]
fn test_optional_credentials_absent() {
    let yaml = r#"
name: sqlite_tool
source:
  type: database
  driver: sqlite
  host: /tmp/data.db
  port: 0
  database: main
  table: results
  columns:
    - token
mapping:
  token: token
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("INGEST_KEY=atc_xyz"));
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    let SourceConfig::Database(ref db) = config.source else { panic!() };
    assert!(db.credentials.is_none());
}

#[test]
fn test_no_env_file_falls_back_to_global_env() {
    unsafe { std::env::set_var("INGEST_KEY", "atc_global") };

    let yaml = r#"
name: tool
source:
  type: file
  format: json
  path: /tmp/a.json
mapping:
  token: value
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, None);
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    assert_eq!(config.atc.ingest_key, "atc_global");
}

// ── load_tool_config_from_dir — mongo ─────────────────────────────────────────

#[test]
fn test_load_mongo_config() {
    let yaml = r#"
name: mongo_tool
source:
  type: mongo
  host: mongo.internal
  port: 27017
  database: nophish
  collection: found_credentials
  filter: '{"exported": false}'
  cursor_field: _id
  credentials:
    username: ${MONGO_USER}
    password: ${MONGO_PASS}
mapping:
  token: credential_value
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(
        yaml,
        Some("MONGO_USER=mongouser\nMONGO_PASS=mongopass\nINGEST_KEY=atc_xyz"),
    );
    let config = load_tool_config_from_dir(dir.path()).unwrap();

    let SourceConfig::Mongo(ref mc) = config.source else {
        panic!("expected Mongo source");
    };
    assert_eq!(mc.collection, "found_credentials");
    assert_eq!(mc.filter.as_deref(), Some(r#"{"exported": false}"#));
    assert_eq!(
        mc.connection_url(),
        "mongodb://mongouser:mongopass@mongo.internal:27017"
    );
}

#[test]
fn test_mongo_connection_url_no_credentials() {
    let mc = MongoConfig {
        host: "localhost".to_string(),
        port: 27017,
        database: "db".to_string(),
        collection: "col".to_string(),
        filter: None,
        cursor_field: None,
        credentials: None,
    };
    assert_eq!(mc.connection_url(), "mongodb://localhost:27017");
}

// ── load_tool_config_from_dir — file / stdout ──────────────────────────────────

#[test]
fn test_load_file_config_json() {
    let yaml = r#"
name: file_tool
source:
  type: file
  format: json
  path: /tmp/results.json
mapping:
  token: value
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("INGEST_KEY=atc_xyz"));
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    let SourceConfig::File(ref fc) = config.source else { panic!() };
    assert_eq!(fc.format, FileFormat::Json);
}

#[test]
fn test_load_stdout_config() {
    let yaml = r#"
name: stdout_tool
source:
  type: stdout
  command: my_tool
  args:
    - --output
    - json
  format: json
mapping:
  token: value
atc:
  url: http://atc.internal
  ingest_key: ${INGEST_KEY}
"#;

    let dir = make_tool_dir(yaml, Some("INGEST_KEY=atc_xyz"));
    let config = load_tool_config_from_dir(dir.path()).unwrap();
    let SourceConfig::Stdout(ref sc) = config.source else { panic!() };
    assert_eq!(sc.command, "my_tool");
}

// ── load_all ───────────────────────────────────────────────────────────────────

#[test]
fn test_load_all_walks_subdirectories() {
    let root = TempDir::new().unwrap();

    let yaml = |name: &str| {
        format!(
            "name: {name}\nsource:\n  type: file\n  format: json\n  path: /tmp/{name}.json\nmapping:\n  token: value\natc:\n  url: http://atc.internal\n  ingest_key: ${{INGEST_KEY}}\n"
        )
    };

    for tool in &["tool_a", "tool_b"] {
        let tool_dir = root.path().join(tool);
        std::fs::create_dir(&tool_dir).unwrap();
        std::fs::write(tool_dir.join("config.yaml"), yaml(tool)).unwrap();
        std::fs::write(tool_dir.join(".env"), "INGEST_KEY=atc_xyz").unwrap();
    }

    std::fs::write(root.path().join("notes.txt"), "ignored").unwrap();

    let mut configs = load_all(root.path()).unwrap();
    configs.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].name, "tool_a");
    assert_eq!(configs[1].name, "tool_b");
}

#[test]
fn test_load_all_skips_subdirs_without_config_yaml() {
    let root = TempDir::new().unwrap();
    std::fs::create_dir(root.path().join("empty_dir")).unwrap();
    let configs = load_all(root.path()).unwrap();
    assert!(configs.is_empty());
}

#[test]
fn test_each_tool_uses_its_own_env() {
    let root = TempDir::new().unwrap();

    let yaml = |name: &str| {
        format!(
            "name: {name}\nsource:\n  type: database\n  driver: postgres\n  host: localhost\n  port: 5432\n  database: db\n  query: \"SELECT token FROM t\"\n  credentials:\n    username: ${{DB_USER}}\n    password: ${{DB_PASS}}\nmapping:\n  token: token\natc:\n  url: http://atc.internal\n  ingest_key: ${{INGEST_KEY}}\n"
        )
    };

    for (tool, user) in &[("tool_a", "user_a"), ("tool_b", "user_b")] {
        let tool_dir = root.path().join(tool);
        std::fs::create_dir(&tool_dir).unwrap();
        std::fs::write(tool_dir.join("config.yaml"), yaml(tool)).unwrap();
        std::fs::write(
            tool_dir.join(".env"),
            format!("DB_USER={user}\nDB_PASS=pass\nINGEST_KEY=atc_xyz"),
        )
        .unwrap();
    }

    let mut configs = load_all(root.path()).unwrap();
    configs.sort_by(|a, b| a.name.cmp(&b.name));

    let SourceConfig::Database(ref db_a) = configs[0].source else { panic!() };
    let SourceConfig::Database(ref db_b) = configs[1].source else { panic!() };

    assert_eq!(db_a.credentials.as_ref().unwrap().username.as_deref(), Some("user_a"));
    assert_eq!(db_b.credentials.as_ref().unwrap().username.as_deref(), Some("user_b"));
}
