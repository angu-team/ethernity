use ethernity_logs::EthernityLogger;
use once_cell::sync::Lazy;

static LOGGER: Lazy<EthernityLogger> = Lazy::new(|| {
    let endpoint = std::env::var("ETHERNITY_LOGS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9200/logs/_doc".to_string());
    EthernityLogger::new(endpoint)
});

pub async fn log_error(message: &str) {
    let _ = LOGGER.log("error", message, "ethernity-simulate").await;
}

pub async fn log_warn(message: &str) {
    let _ = LOGGER.log("warn", message, "ethernity-simulate").await;
}
