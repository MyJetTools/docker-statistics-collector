use std::fmt;

#[derive(Debug, Clone)]
pub struct RequestError {
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(err: reqwest::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}
