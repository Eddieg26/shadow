use crate::asset::AssetId;
use std::{error::Error, fmt::Display, path::PathBuf};

#[derive(Debug)]
pub enum AssetError {
    AssetNotFound(AssetId),
    InvalidPath(PathBuf),
    InvalidExtension(PathBuf),
    InvalidMetadata,
    Io(std::io::Error),
}

impl From<std::io::Error> for AssetError {
    fn from(error: std::io::Error) -> Self {
        AssetError::Io(error)
    }
}

impl From<AssetError> for std::io::Error {
    fn from(error: AssetError) -> std::io::Error {
        match error {
            AssetError::AssetNotFound(id) => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found {:?}", id),
            ),
            AssetError::InvalidPath(_) => {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path")
            }
            AssetError::InvalidExtension(_) => {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid extension")
            }
            AssetError::InvalidMetadata => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid metadata")
            }
            AssetError::Io(error) => error,
        }
    }
}

impl Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::AssetNotFound(id) => f.write_fmt(format_args!("Asset Not found: {:?}", id)),
            AssetError::InvalidPath(path) => f.write_fmt(format_args!("Invalid Path: {:?}", path)),
            AssetError::InvalidExtension(path) => {
                f.write_fmt(format_args!("Invalid Path: {:?}", path))
            }
            AssetError::InvalidMetadata => f.write_str("Invalid Metadata"),
            AssetError::Io(error) => {
                let error = error.to_string();
                f.write_fmt(format_args!("IO Asset Error: {:?}", error))
            }
        }
    }
}

impl Error for AssetError {}
