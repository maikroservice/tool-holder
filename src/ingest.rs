use std::collections::HashMap;

use serde_json::{Value, json};

use crate::error::IngestError;

pub struct AtcClient {
    url: String,
    ingest_key: String,
    client: reqwest::Client,
}

impl AtcClient {
    pub fn new(url: String, ingest_key: String) -> Result<Self, IngestError> {
        let client = reqwest::Client::builder().build()?;
        Ok(Self { url, ingest_key, client })
    }

    /// POST one credential record to ATC `/ingest`.
    pub async fn ingest(
        &self,
        source: &str,
        credentials: HashMap<String, Value>,
    ) -> Result<(), IngestError> {
        let payload = json!({
            "source": source,
            "credentials": credentials,
        });

        let response = self
            .client
            .post(format!("{}/ingest", self.url))
            .header("x-ingest-key", &self.ingest_key)
            .json(&payload)
            .send()
            .await?;

        match response.status().as_u16() {
            200 | 201 => Ok(()),
            403 => Err(IngestError::Unauthorized),
            status => Err(IngestError::Server { status }),
        }
    }
}
