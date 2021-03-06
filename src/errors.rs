use std::fmt::{Display, Formatter};
use std::error::Error;

#[derive(Debug)]
pub enum ScrapeError {
    NetworkError(String),
}

impl Display for ScrapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrapeError::NetworkError(err) => {
                write!(f, "Scraping network error: {}", err)
            }
        }
    }
}

impl Error for ScrapeError {}

impl From<reqwest::Error> for ScrapeError {
    fn from(err: reqwest::Error) -> Self {
        ScrapeError::NetworkError(err.to_string())
    }
}

