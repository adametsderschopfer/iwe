//! Describe a verified release without transport or filesystem access.

#[derive(Debug, Clone)]
pub(crate) struct Release {
    pub version: String,
    pub tag: String,
    pub asset: String,
    pub size: u64,
    pub sha256: String,
    pub manifest: Vec<u8>,
    pub signature: Vec<u8>,
}
