//! Public authlib-injector artifact metadata and checksum verification.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const OFFICIAL_LATEST: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";

/// Shape of `latest.json` (the panel mirror uses the same format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub build_number: u64,
    pub version: String,
    pub download_url: String,
    pub checksums: Checksums,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksums {
    pub sha256: String,
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}
