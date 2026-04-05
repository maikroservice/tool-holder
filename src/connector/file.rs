use std::collections::HashMap;
use std::fs;

use async_trait::async_trait;
use serde_json::Value;

use crate::config::{FileConfig, FileFormat};
use crate::connector::{Connector, Row};
use crate::error::ConnectorError;

pub struct FileConnector {
    config: FileConfig,
}

impl FileConnector {
    pub fn new(config: FileConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Connector for FileConnector {
    async fn fetch(&self) -> Result<Vec<Row>, ConnectorError> {
        let content = fs::read_to_string(&self.config.path)?;
        parse_content(&content, self.config.format)
    }
}

pub fn parse_content(content: &str, format: FileFormat) -> Result<Vec<Row>, ConnectorError> {
    match format {
        FileFormat::Json => {
            let val: Value = serde_json::from_str(content)
                .map_err(|e| ConnectorError::Parse(e.to_string()))?;
            rows_from_json(val)
        }
        FileFormat::Yaml => {
            let val: Value = serde_yaml::from_str(content)
                .map_err(|e| ConnectorError::Parse(e.to_string()))?;
            rows_from_json(val)
        }
        FileFormat::Txt => {
            let rows = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|line| {
                    let mut row = HashMap::new();
                    row.insert("line".to_string(), Value::String(line.to_string()));
                    row
                })
                .collect();
            Ok(rows)
        }
    }
}

/// Accepts either a JSON array of objects or a single object (wrapped in a vec).
fn rows_from_json(val: Value) -> Result<Vec<Row>, ConnectorError> {
    match val {
        Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                Value::Object(map) => Ok(map.into_iter().collect()),
                _ => Err(ConnectorError::Parse(
                    "expected array of objects".to_string(),
                )),
            })
            .collect(),
        Value::Object(map) => Ok(vec![map.into_iter().collect()]),
        _ => Err(ConnectorError::Parse(
            "expected JSON object or array of objects".to_string(),
        )),
    }
}
