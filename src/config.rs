use std::collections::HashMap;
use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;

use crate::error::ConfigError;

/// One tool config loaded from a tool directory.
#[derive(Debug, Deserialize, PartialEq)]
pub struct ToolConfig {
    /// Identifier sent as `source` in the ATC ingest payload.
    pub name: String,
    pub source: SourceConfig,
    /// Maps ATC field names → tool field names.
    pub mapping: HashMap<String, String>,
    pub atc: AtcConfig,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceConfig {
    /// SQL database — postgres, sqlite, or mysql.
    Database(DatabaseConfig),
    /// MongoDB collection.
    Mongo(MongoConfig),
    /// Local file — JSON, YAML, or plain text.
    File(FileConfig),
    /// Capture stdout of a command.
    Stdout(StdoutConfig),
}

// ── SQL database ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    Postgres,
    Sqlite,
    Mysql,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub host: String,
    pub port: u16,
    pub database: String,
    /// Mutually exclusive with `query`. Requires `columns` when set.
    pub table: Option<String>,
    pub columns: Option<Vec<String>>,
    /// Raw SQL query — takes precedence over table + columns.
    pub query: Option<String>,
    /// Column used to track the last seen row (watermark).
    pub cursor_field: Option<String>,
    pub credentials: Option<Credentials>,
}

impl DatabaseConfig {
    /// Build a connection URL from config fields.
    pub fn connection_url(&self) -> String {
        match self.driver {
            DatabaseDriver::Sqlite => format!("sqlite://{}", self.host),
            DatabaseDriver::Postgres => match &self.credentials {
                Some(c) => format!(
                    "postgres://{}:{}@{}:{}/{}",
                    c.username.as_deref().unwrap_or(""),
                    c.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                ),
                None => format!("postgres://{}:{}/{}", self.host, self.port, self.database),
            },
            DatabaseDriver::Mysql => match &self.credentials {
                Some(c) => format!(
                    "mysql://{}:{}@{}:{}/{}",
                    c.username.as_deref().unwrap_or(""),
                    c.password.as_deref().unwrap_or(""),
                    self.host,
                    self.port,
                    self.database
                ),
                None => format!("mysql://{}:{}/{}", self.host, self.port, self.database),
            },
        }
    }

    /// Returns Err if neither table+columns nor query is provided.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let has_table = self.table.is_some() && self.columns.is_some();
        let has_query = self.query.is_some();
        if !has_table && !has_query {
            return Err(ConfigError::InvalidDatabaseSource);
        }
        Ok(())
    }
}

// ── MongoDB ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
pub struct MongoConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub collection: String,
    /// Optional JSON filter document e.g. `{"exported": false}`.
    pub filter: Option<String>,
    /// Field used as watermark cursor.
    pub cursor_field: Option<String>,
    pub credentials: Option<Credentials>,
}

impl MongoConfig {
    pub fn connection_url(&self) -> String {
        match &self.credentials {
            Some(c) => format!(
                "mongodb://{}:{}@{}:{}",
                c.username.as_deref().unwrap_or(""),
                c.password.as_deref().unwrap_or(""),
                self.host,
                self.port
            ),
            None => format!("mongodb://{}:{}", self.host, self.port),
        }
    }
}

// ── File ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
pub struct FileConfig {
    pub format: FileFormat,
    pub path: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Json,
    Yaml,
    Txt,
}

// ── Stdout ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
pub struct StdoutConfig {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub format: Option<FileFormat>,
}

// ── Shared ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, PartialEq)]
pub struct Credentials {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AtcConfig {
    pub url: String,
    pub ingest_key: String,
}

// ── Loading ────────────────────────────────────────────────────────────────────

/// Load a tool config from a directory containing `config.yaml` and optionally `.env`.
///
/// Variables in the YAML are substituted from the tool's `.env` first,
/// then falling back to the process environment.
pub fn load_tool_config_from_dir(dir: impl AsRef<Path>) -> Result<ToolConfig, ConfigError> {
    let dir = dir.as_ref();
    let local_env = load_local_env(dir)?;
    let raw = fs::read_to_string(dir.join("config.yaml"))?;
    let substituted = substitute_env_vars(&raw, &local_env)?;
    let config: ToolConfig = serde_yaml::from_str(&substituted)?;
    if let SourceConfig::Database(ref db) = config.source {
        db.validate()?;
    }
    Ok(config)
}

/// Load all tool subdirectories from `dir`.
/// Each subdirectory that contains a `config.yaml` is treated as a tool.
pub fn load_all(dir: impl AsRef<Path>) -> Result<Vec<ToolConfig>, ConfigError> {
    let mut configs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("config.yaml").exists() {
            configs.push(load_tool_config_from_dir(&path)?);
        }
    }
    Ok(configs)
}

/// Load a tool's `.env` file into a local map. Returns an empty map if absent.
/// The local map is used for variable substitution before falling back to global env.
fn load_local_env(dir: &Path) -> Result<HashMap<String, String>, ConfigError> {
    let env_path = dir.join(".env");
    if !env_path.exists() {
        return Ok(HashMap::new());
    }
    let map = dotenvy::from_path_iter(&env_path)
        .map_err(|e| ConfigError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

/// Replace every `${VAR_NAME}` in `raw`.
/// Lookup order: `local_env` → process environment.
/// Returns an error listing all missing variables at once.
pub fn substitute_env_vars(
    raw: &str,
    local_env: &HashMap<String, String>,
) -> Result<String, ConfigError> {
    let re = Regex::new(r"\$\{([^}]+)\}").expect("static regex is valid");
    let mut missing = Vec::new();

    let result = re.replace_all(raw, |caps: &regex::Captures| {
        let var = &caps[1];
        if let Some(val) = local_env.get(var) {
            return val.clone();
        }
        match std::env::var(var) {
            Ok(val) => val,
            Err(_) => {
                missing.push(var.to_string());
                String::new()
            }
        }
    });

    if !missing.is_empty() {
        return Err(ConfigError::MissingEnvVar(missing.join(", ")));
    }

    Ok(result.into_owned())
}
