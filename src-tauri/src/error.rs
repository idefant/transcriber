use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
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
