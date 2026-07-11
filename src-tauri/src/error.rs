use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    /// Ошибка, возвращённая внешним API. `message` — краткая понятная человеку
    /// сводка (например, "STT request failed with status 401"); `details` хранит
    /// исходное тело ответа, разобранное как JSON, если это возможно, для изучения в UI.
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

    /// Строит ошибку API из краткого резюме и исходного тела ответа. Тело
    /// сохраняется как разобранный JSON, если это валидный JSON, иначе как обычная строка.
    pub fn api(message: String, body: &str) -> Self {
        let details = serde_json::from_str(body)
            .ok()
            .or_else(|| Some(serde_json::Value::String(body.to_string())));

        Self::Api { message, details }
    }

    /// Разделяет ошибку на отображаемое сообщение и необязательные структурированные детали.
    /// Только ошибки `Api` содержат детали; любой другой вариант возвращает `None`.
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
