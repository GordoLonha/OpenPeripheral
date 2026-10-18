use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("hid: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("device not found: {0}")]
    NotFound(&'static str),
    #[error("no reply from device (timeout)")]
    Timeout,
    #[error("unexpected reply: {0}")]
    BadReply(String),
}

pub type Result<T> = std::result::Result<T, Error>;
