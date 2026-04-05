use async_trait::async_trait;
use bson::{Bson, Document};
use futures::TryStreamExt;
use mongodb::Client;
use serde_json::{Value, json};

use crate::config::MongoConfig;
use crate::connector::{Connector, Row};
use crate::error::ConnectorError;

pub struct MongoConnector {
    client: Client,
    config: MongoConfig,
}

impl MongoConnector {
    pub async fn new(config: MongoConfig) -> Result<Self, ConnectorError> {
        let client = Client::with_uri_str(&config.connection_url())
            .await
            .map_err(|e| ConnectorError::Database(e.to_string()))?;
        Ok(Self { client, config })
    }
}

#[async_trait]
impl Connector for MongoConnector {
    async fn fetch(&self) -> Result<Vec<Row>, ConnectorError> {
        let db = self.client.database(&self.config.database);
        let collection: mongodb::Collection<Document> = db.collection(&self.config.collection);

        let filter = match self.config.filter.as_deref() {
            Some(json_str) => {
                let val: Value = serde_json::from_str(json_str)
                    .map_err(|e| ConnectorError::Parse(e.to_string()))?;
                bson::to_document(&val)
                    .map_err(|e| ConnectorError::Parse(e.to_string()))?
            }
            None => Document::new(),
        };

        let mut cursor = collection
            .find(filter)
            .await
            .map_err(|e| ConnectorError::Database(e.to_string()))?;

        let mut rows = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| ConnectorError::Database(e.to_string()))?
        {
            rows.push(document_to_row(doc));
        }
        Ok(rows)
    }
}

/// Convert a BSON document to a `Row`, mapping BSON-specific types to JSON.
fn document_to_row(doc: Document) -> Row {
    doc.into_iter()
        .map(|(k, v)| (k, bson_to_json(v)))
        .collect()
}

fn bson_to_json(bson: Bson) -> Value {
    match bson {
        Bson::Double(f) => json!(f),
        Bson::String(s) => json!(s),
        Bson::Boolean(b) => json!(b),
        Bson::Int32(i) => json!(i),
        Bson::Int64(i) => json!(i),
        Bson::Null => Value::Null,
        Bson::Array(arr) => Value::Array(arr.into_iter().map(bson_to_json).collect()),
        Bson::Document(doc) => {
            let map = doc.into_iter().map(|(k, v)| (k, bson_to_json(v))).collect();
            Value::Object(map)
        }
        // Represent BSON-specific types as strings so they survive the mapping step.
        Bson::ObjectId(oid) => json!(oid.to_string()),
        Bson::DateTime(dt) => json!(dt.to_string()),
        Bson::Timestamp(ts) => json!(format!("{}/{}", ts.time, ts.increment)),
        Bson::Binary(b) => json!(hex::encode(b.bytes)),
        other => json!(other.to_string()),
    }
}
