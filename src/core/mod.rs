pub mod config;
pub mod error;
pub mod models;
pub mod patent_search;

pub use error::{Error, Result};

// Re-export chrome-cdp types for backward compatibility
pub use chrome_cdp::{BrowserManager, CdpBrowser, CdpPage};
