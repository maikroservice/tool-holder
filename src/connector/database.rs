use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::any::AnyPoolOptions;
use sqlx::{AnyPool, Column, Row as SqlxRow};

use crate::config::DatabaseConfig;
use crate::connector::{Connector, Row};
use crate::error::ConnectorError;

pub struct DatabaseConnector {
    pool: AnyPool,
    config: DatabaseConfig,
}

impl DatabaseConnector {
    pub async fn new(config: DatabaseConfig) -> Result<Self, ConnectorError> {
        // Must be called once before using AnyPool — safe to call multiple times.
        sqlx::any::install_default_drivers();

        let pool = AnyPoolOptions::new()
            .connect(&config.connection_url())
            .await
            .map_err(|e| ConnectorError::Database(e.to_string()))?;

        Ok(Self { pool, config })
    }

    fn build_query(&self) -> Result<String, ConnectorError> {
        if let Some(ref raw_query) = self.config.query {
            return Ok(raw_query.clone());
        }

        let table = self.config.table.as_ref()
            .ok_or_else(|| ConnectorError::Database("No table or query configured".into()))?;
        let columns = self.config.columns.as_ref()
            .ok_or_else(|| ConnectorError::Database("No columns configured".into()))?;

        let mut select_cols = columns.join(", ");
        if let Some(ref cursor) = self.config.cursor_field {
            if !columns.contains(cursor) {
                select_cols = format!("{}, {}", select_cols, cursor);
            }
        }

        Ok(format!("SELECT {} FROM {}", select_cols, table))
    }
}

#[async_trait]
impl Connector for DatabaseConnector {
    async fn fetch(&self) -> Result<Vec<Row>, ConnectorError> {
        let sql = self.build_query()?;
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ConnectorError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let mut map: Row = HashMap::new();
            for col in row.columns() {
                // Try each type in descending specificity — AnyRow does not carry
                // static type info, so we fall through until something succeeds.
                let value = if let Ok(v) = row.try_get::<i64, _>(col.ordinal()) {
                    Value::from(v)
                } else if let Ok(v) = row.try_get::<f64, _>(col.ordinal()) {
                    Value::from(v)
                } else if let Ok(v) = row.try_get::<bool, _>(col.ordinal()) {
                    Value::Bool(v)
                } else if let Ok(v) = row.try_get::<String, _>(col.ordinal()) {
                    Value::String(v)
                } else {
                    Value::Null
                };
                map.insert(col.name().to_string(), value);
            }
            result.push(map);
        }
        Ok(result)
    }
}
