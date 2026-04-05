use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::ConnectorError;

pub mod database;
pub mod file;
pub mod mongo;
pub mod stdout;

/// A single record returned by a connector — field name → JSON value.
pub type Row = HashMap<String, Value>;

#[async_trait]
pub trait Connector: Send + Sync {
    async fn fetch(&self) -> Result<Vec<Row>, ConnectorError>;
}
