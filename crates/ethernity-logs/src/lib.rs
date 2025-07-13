use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Serialize;
use thiserror::Error;

/// Tipo de erro retornado pelo logger.
#[derive(Debug, Error)]
pub enum LogError {
    #[error("erro ao enviar log: {0}")]
    Request(#[from] reqwest::Error),
}

/// Estrutura de log enviada para o Elasticsearch.
#[derive(Serialize)]
struct LogEntry<'a> {
    level: &'a str,
    message: &'a str,
    crate_name: &'a str,
    timestamp: DateTime<Utc>,
}

/// Cliente simples para envio de logs ao Elasticsearch.
pub struct EthernityLogger {
    endpoint: String,
    client: Client,
}

impl EthernityLogger {
    /// Cria uma nova instância apontando para a `endpoint` do Elasticsearch.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: Client::new(),
        }
    }

    /// Envia um log para o Elasticsearch.
    pub async fn log(
        &self,
        level: &str,
        message: &str,
        crate_name: &str,
    ) -> Result<(), LogError> {
        let entry = LogEntry {
            level,
            message,
            crate_name,
            timestamp: Utc::now(),
        };
        self.client
            .post(&self.endpoint)
            .json(&entry)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn send_log_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let logger = EthernityLogger::new(server.uri());
        let result = logger.log("info", "test", "crate").await;
        assert!(result.is_ok());
    }
}
