use std::env;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error(transparent)]
    Var(#[from] env::VarError),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    ReqwestParse(#[from] url::ParseError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error("scraper error: {0}")]
    Scraper(String),
}

impl<'a> From<scraper::error::SelectorErrorKind<'a>> for AppError {
    fn from(error: scraper::error::SelectorErrorKind<'a>) -> Self {
        Self::Scraper(error.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
