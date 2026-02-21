use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Browser error: {0}")]
    Browser(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    #[error("MCP protocol error: {0}")]
    McpProtocol(#[from] mcp_sdk_rs::error::Error),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(Box::new(err))
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}
