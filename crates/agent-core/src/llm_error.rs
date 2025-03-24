use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("wrong llm response")]
    WrongLlmResponse { response: String },
    #[error("reqwest error: {0:#?}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("io error: {0:#?}")]
    IoError(#[from] std::io::Error),
    #[error("json deserialization error: {0:#?}")]
    JsonDeserializationError(#[from] serde_json::Error),
}
