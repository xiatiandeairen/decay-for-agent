use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecayError {
    #[error("io error at {path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },

    #[error("database error: {message}")]
    Db {
        message: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("invalid project: {0}")]
    InvalidProject(String),
}

pub type Result<T> = std::result::Result<T, DecayError>;
