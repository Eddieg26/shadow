use crate::asset::AssetId;
use std::{error::Error, fmt::Display, path::PathBuf};

#[derive(Debug)]
pub enum AssetError {
    Loading {
        id: AssetId,
        message: String,
    },
    Importing {
        path: PathBuf,
        message: String,
    },
    Processing {
        id: AssetId,
        message: String,
    },
    PostProcessing {
        id: AssetId,
        message: String,
    },
    Saving {
        id: AssetId,
        path: PathBuf,
        message: String,
    },
}

impl Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Importing { path, message } => {
                write!(f, "Failed to import asset at {:?}: {}", path, message)
            }
            AssetError::Loading { id, message } => {
                write!(f, "Failed to load asset {:?}: {}", id, message)
            }
            AssetError::Processing { id, message } => {
                write!(f, "Failed to process asset {:?}: {}", id, message)
            }
            AssetError::PostProcessing { id, message } => {
                write!(f, "Failed to post-process asset {:?}: {}", id, message)
            }
            AssetError::Saving { id, path, message } => {
                write!(
                    f,
                    "Failed to save asset {:?} at {:?}: {}",
                    id, path, message
                )
            }
        }
    }
}

impl Error for AssetError {}
