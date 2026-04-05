use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid config — database source requires either 'table' + 'columns' or 'query'")]
    InvalidDatabaseSource,
}

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("File error: {0}")]
    File(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Command error: {0}")]
    Command(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Unauthorized — invalid or revoked ingest key (HTTP 403)")]
    Unauthorized,
    #[error("ATC server error (HTTP {status})")]
    Server { status: u16 },
}
