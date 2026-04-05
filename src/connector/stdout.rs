use async_trait::async_trait;
use tokio::process::Command;

use crate::config::{FileFormat, StdoutConfig};
use crate::connector::{Connector, Row};
use crate::connector::file::parse_content;
use crate::error::ConnectorError;

pub struct StdoutConnector {
    config: StdoutConfig,
}

impl StdoutConnector {
    pub fn new(config: StdoutConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Connector for StdoutConnector {
    async fn fetch(&self) -> Result<Vec<Row>, ConnectorError> {
        let mut cmd = Command::new(&self.config.command);
        if let Some(ref args) = self.config.args {
            cmd.args(args);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| ConnectorError::Command(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ConnectorError::Command(format!(
                "command exited with {}: {}",
                output.status, stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let format = self.config.format.unwrap_or(FileFormat::Txt);
        parse_content(&stdout, format)
    }
}
