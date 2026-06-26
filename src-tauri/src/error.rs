use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    /// An error returned by an external API. `message` is a short, human-readable
    /// summary (e.g. "STT request failed with status 401"); `details` holds the
    /// raw response body, parsed as JSON when possible, for inspection in the UI.
    #[error("{message}")]
    Api {
        message: String,
        details: Option<serde_json::Value>,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Path(#[from] tauri::Error),
}

impl AppError {
    pub fn into_message(self) -> String {
        self.to_string()
    }

    /// Build an API error from a short summary and the raw response body. The body
    /// is stored as parsed JSON when it is valid JSON, otherwise as a plain string.
    pub fn api(message: String, body: &str) -> Self {
        let details = serde_json::from_str(body)
            .ok()
            .or_else(|| Some(serde_json::Value::String(body.to_string())));

        Self::Api { message, details }
    }

    /// Split the error into its display message and optional structured details.
    /// Only `Api` errors carry details; every other variant returns `None`.
    pub fn into_message_and_details(self) -> (String, Option<serde_json::Value>) {
        match self {
            Self::Api { message, details } => (message, details),
            other => (other.to_string(), None),
        }
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}
