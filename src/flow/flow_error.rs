use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlowError {
    #[error("Node Id not found")]
    InvalidNodeIdError,
    #[error("Node IO Index not found")]
    InvalidNodeIOIndexError,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
