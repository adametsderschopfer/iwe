//! Download signed releases from the fixed repository without account credentials.

mod install;
#[cfg(test)]
mod tests;

use crate::domain::update::Release;
use ed25519_dalek::{Signature, VerifyingKey};
use reqwest::{Client, redirect};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

pub(crate) use install::{Prepared, start_install, startup};
const REPOSITORY: &str = "https://github.com/adametsderschopfer/iwe-code";
const LATEST: &str = "https://api.github.com/repos/adametsderschopfer/iwe-code/releases/latest";
const MAX_PACKAGE: u64 = 512 * 1024 * 1024;
const PUBLIC_KEY: &[u8; 32] = include_bytes!("../../../assets/update-public-key.bin");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Network,
    Invalid,
    Install,
    Unsupported,
}

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}
#[derive(Deserialize)]
struct Manifest {
    version: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    sha256: String,
}

pub(crate) fn platform() -> Result<&'static str, Error> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("macos", "x86_64") => Ok("macos-x64"),
        ("windows", "x86_64") => Ok("windows-x64"),
        ("windows", "x86") => Ok("windows-x86"),
        ("linux", "x86_64") => Ok("linux-x64"),
        ("linux", "x86") => Ok("linux-x86"),
        _ => Err(Error::Unsupported),
    }
}

fn client() -> Result<Client, Error> {
    Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("IWE-Code/", env!("CARGO_PKG_VERSION")))
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 5 && allowed_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| Error::Network)
}

fn allowed_url(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "api.github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn asset_url(tag: &str, name: &str) -> String {
    format!("{REPOSITORY}/releases/download/{tag}/{name}")
}

async fn small(client: &Client, url: &str, limit: usize) -> Result<Vec<u8>, Error> {
    let mut response = client.get(url).send().await.map_err(|_| Error::Network)?;
    if !response.status().is_success() {
        return Err(Error::Network);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
        if bytes.len() + chunk.len() > limit {
            return Err(Error::Invalid);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(crate) async fn check() -> Result<Option<Release>, Error> {
    tokio::time::timeout(Duration::from_secs(45), async {
        let client = client()?;
        let latest: Latest = serde_json::from_slice(&small(&client, LATEST, 64 * 1024).await?)
            .map_err(|_| Error::Invalid)?;
        if latest.draft || latest.prerelease || !newer(&latest.tag_name, env!("CARGO_PKG_VERSION"))?
        {
            return Ok(None);
        }
        let manifest = small(
            &client,
            &asset_url(&latest.tag_name, "updates.json"),
            64 * 1024,
        )
        .await?;
        let signature = small(&client, &asset_url(&latest.tag_name, "updates.sig"), 64).await?;
        verified_release(
            &manifest,
            &signature,
            &latest.tag_name,
            platform()?,
            PUBLIC_KEY,
        )
        .map(Some)
    })
    .await
    .map_err(|_| Error::Network)?
}

fn newer(tag: &str, current: &str) -> Result<bool, Error> {
    let version =
        Version::parse(tag.strip_prefix('v').unwrap_or(tag)).map_err(|_| Error::Invalid)?;
    let current = Version::parse(current).map_err(|_| Error::Invalid)?;
    // Reject unsafe URL components and every prerelease channel.
    if !tag
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
        || !version.pre.is_empty()
    {
        return Err(Error::Invalid);
    }
    Ok(version > current)
}

fn verified_release(
    bytes: &[u8],
    signature: &[u8],
    tag: &str,
    platform: &str,
    public: &[u8; 32],
) -> Result<Release, Error> {
    VerifyingKey::from_bytes(public)
        .map_err(|_| Error::Invalid)?
        .verify_strict(
            bytes,
            &Signature::from_slice(signature).map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?;
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
    let version = tag.strip_prefix('v').unwrap_or(tag);
    if manifest.version != version || manifest.assets.len() != 6 {
        return Err(Error::Invalid);
    }
    let name = format!("iwe-{version}-{platform}-update.tar.gz");
    let mut assets = manifest
        .assets
        .into_iter()
        .filter(|asset| asset.name == name);
    let asset = assets.next().ok_or(Error::Invalid)?;
    if assets.next().is_some()
        || asset.size == 0
        || asset.size > MAX_PACKAGE
        || asset.sha256.len() != 64
        || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Error::Invalid);
    }
    Ok(Release {
        version: version.to_owned(),
        tag: tag.to_owned(),
        asset: name,
        size: asset.size,
        sha256: asset.sha256.to_ascii_lowercase(),
        manifest: bytes.to_vec(),
        signature: signature.to_vec(),
    })
}

pub(crate) async fn download(release: Release) -> Result<Arc<Prepared>, Error> {
    let target = install::current_target()?;
    // Development binaries must never replace a checkout's executable.
    if cfg!(debug_assertions) {
        return Err(Error::Unsupported);
    }
    let verified = verified_release(
        &release.manifest,
        &release.signature,
        &release.tag,
        platform()?,
        PUBLIC_KEY,
    )?;
    let target_copy = target.clone();
    let stage = tokio::task::spawn_blocking(move || install::stage_directory(&target_copy))
        .await
        .map_err(|_| Error::Install)??;
    let archive = stage.path().join("package.tar.gz");
    let mut output = tokio::fs::File::create(&archive)
        .await
        .map_err(|_| Error::Install)?;
    let mut response = client()?
        .get(asset_url(&verified.tag, &verified.asset))
        .send()
        .await
        .map_err(|_| Error::Network)?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length != verified.size)
    {
        return Err(Error::Invalid);
    }
    let mut total = 0_u64;
    let mut digest = Sha256::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
        total += chunk.len() as u64;
        if total > verified.size {
            return Err(Error::Invalid);
        }
        digest.update(&chunk);
        output.write_all(&chunk).await.map_err(|_| Error::Install)?;
    }
    output.sync_all().await.map_err(|_| Error::Install)?;
    drop(output);
    if total != verified.size || format!("{:x}", digest.finalize()) != verified.sha256 {
        return Err(Error::Invalid);
    }
    tokio::task::spawn_blocking(move || install::prepare(stage, target, verified))
        .await
        .map_err(|_| Error::Install)?
        .map(Arc::new)
}

fn read_bounded(path: impl AsRef<std::path::Path>, limit: u64) -> Result<Vec<u8>, Error> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| Error::Install)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Install)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Invalid);
    }
    Ok(bytes)
}
