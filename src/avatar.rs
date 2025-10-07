//! Avatar fetching and caching utilities.
//!
//! This module provides helper functions to download profile avatars while reusing
//! the same caching patterns as the [`network`] module. It is only compiled when
//! the `fetch` feature flag is enabled, as it depends on asynchronous HTTP
//! fetching via `reqwest` and `tokio`.

use crate::network::FetchOptions;
use crate::profile::Profile;
use reqwest::header::CONTENT_TYPE;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

/// Returns a conservative default [`FetchOptions`] suitable for rarely changing avatars.
///
/// * Fetch timeout defaults to 30 seconds.
/// * Cached avatars remain valid for 30 days.
/// * Cache directory can be supplied via the argument.
pub fn default_avatar_fetch_options(cache_store: Option<PathBuf>) -> FetchOptions {
    FetchOptions {
        fetch_timeout: Some(Duration::from_secs(30)),
        cache_timeout: Some(Duration::from_secs(60 * 60 * 24 * 30)),
        cache_store,
    }
}

/// Downloaded avatar payload together with basic metadata.
#[derive(Debug, Clone)]
pub struct AvatarData {
    /// Source URL of the avatar.
    pub url: String,
    /// Raw binary bytes for the avatar image.
    pub bytes: Vec<u8>,
    /// Optional HTTP content type returned by the server.
    pub content_type: Option<String>,
}

impl AvatarData {
    fn new(url: String, bytes: Vec<u8>, content_type: Option<String>) -> Self {
        Self {
            url,
            bytes,
            content_type,
        }
    }
}

/// In-memory mapping for avatar lookup keyed by profile nickname.
///
/// The nickname is treated as the canonical identifier for avatar lookups.
/// When a profile lacks a nickname, the display title is used as a fallback.
#[derive(Debug, Default, Clone)]
pub struct ProfileAvatarMap {
    inner: HashMap<String, AvatarData>,
}

impl ProfileAvatarMap {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Inserts an avatar for a custom key.
    pub fn insert<K: Into<String>>(&mut self, key: K, avatar: AvatarData) {
        self.inner.insert(key.into(), avatar);
    }

    /// Inserts an avatar using the derived key for the provided profile.
    pub fn insert_profile(&mut self, profile: &Profile, avatar: AvatarData) {
        let key = profile.nick().to_string();
        self.inner.insert(key, avatar);
    }

    /// Retrieves an avatar using a custom key.
    pub fn get(&self, key: &str) -> Option<&AvatarData> {
        self.inner.get(key)
    }

    /// Retrieves an avatar for the provided profile using the derived key.
    pub fn get_for_profile(&self, profile: &Profile) -> Option<&AvatarData> {
        let key = profile.nick().to_string();
        self.inner.get(&key)
    }

    /// Number of entries within the mapping.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the mapping currently holds no avatars.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterator over all stored avatars.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AvatarData)> {
        self.inner.iter()
    }
}

const SUPPORTED_EXTENSIONS: [&str; 2] = [".jpg", ".png"];

/// Fetches the avatar for a single profile.
///
/// The function first checks the cache. If no valid cache is found, it performs
/// a network request using the supplied `client`. Should the fetch fail, the
/// function will fall back to any cached avatar regardless of age.
pub async fn fetch_avatar_for_profile_with_client(
    client: &reqwest::Client,
    profile: &Profile,
    options: &FetchOptions,
) -> Result<Option<AvatarData>, Box<dyn std::error::Error + Send + Sync>> {
    let avatar_url = match profile.avatar() {
        Some(url) if !url.trim().is_empty() => url.trim().to_string(),
        _ => return Ok(None),
    };

    if !is_supported_avatar_url(&avatar_url) {
        return Ok(None);
    }

    if let Ok(Some(bytes)) = get_cached_avatar(&avatar_url, options.cache_timeout, &options.cache_store).await {
        return Ok(Some(AvatarData::new(avatar_url.clone(), bytes, None)));
    }

    let request_future = async {
        let response = client.get(&avatar_url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error {}: {}", response.status(), avatar_url).into());
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_string());

        if let Some(ref ct) = content_type {
            if !ct.starts_with("image/") {
                return Err(format!("Unsupported content type {ct} for {avatar_url}").into());
            }
        }

        let bytes = response.bytes().await?.to_vec();

        let url_owned = avatar_url.clone();
        let cache_store = options.cache_store.clone();
        let bytes_clone = bytes.clone();

        tokio::spawn(async move {
            let _ = save_to_cache(&url_owned, &bytes_clone, &cache_store).await;
        });

        Ok((bytes, content_type))
    };

    let fetch_result = match options.fetch_timeout {
        Some(duration) => match timeout(duration, request_future).await {
            Ok(result) => result,
            Err(_) => Err(format!("Timeout after {duration:?} while fetching {avatar_url}").into()),
        },
        None => request_future.await,
    };

    match fetch_result {
        Ok((bytes, content_type)) => Ok(Some(AvatarData::new(avatar_url, bytes, content_type))),
        Err(err) => {
            if let Ok(Some(bytes)) = get_cached_avatar_fallback(&avatar_url, &options.cache_store).await {
                Ok(Some(AvatarData::new(avatar_url, bytes, None)))
            } else {
                Err(err)
            }
        }
    }
}

/// Convenience wrapper that builds a [`reqwest::Client`] internally.
pub async fn fetch_avatar_for_profile(
    profile: &Profile,
    options: FetchOptions,
) -> Result<Option<AvatarData>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    fetch_avatar_for_profile_with_client(&client, profile, &options).await
}

/// Fetches avatars for a list of profiles concurrently.
///
/// The resulting [`ProfileAvatarMap`] will contain entries for every profile
/// that has an avatar URL and could be fetched successfully. Profiles without
/// avatar URLs or failing to fetch are skipped.
pub async fn fetch_avatars_for_profiles(
    profiles: Vec<Profile>,
    options: FetchOptions,
) -> ProfileAvatarMap {
    use std::sync::Arc;

    let client = Arc::new(reqwest::Client::new());
    let options = Arc::new(options);

    let tasks: Vec<_> = profiles
        .into_iter()
        .map(|profile| {
            let client = client.clone();
            let options = options.clone();
            tokio::spawn(async move {
                let key = profile.nick().to_string();
                match fetch_avatar_for_profile_with_client(&client, &profile, &options).await {
                    Ok(Some(avatar)) => Some((key, avatar)),
                    Ok(None) => None,
                    Err(err) => {
                        eprintln!(
                            "Failed to fetch avatar for {}: {err}",
                            profile.nick()
                        );
                        None
                    }
                }
            })
        })
        .collect();

    let mut map = ProfileAvatarMap::new();

    for task in tasks {
        match task.await {
            Ok(Some((key, avatar))) => {
                map.insert(key, avatar);
            }
            Ok(None) => {}
            Err(join_err) => {
                eprintln!("Avatar fetch task panicked: {join_err}");
            }
        }
    }

    map
}

fn is_supported_avatar_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(ext))
}

async fn get_cached_avatar(
    url: &str,
    cache_timeout: Option<Duration>,
    cache_store: &Option<PathBuf>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(None),
    };

    let cache_file = get_cache_file_path(cache_store, url);

    if !cache_file.exists() {
        return Ok(None);
    }

    let timeout = match cache_timeout {
        Some(value) => value,
        None => return Ok(None),
    };

    let metadata = fs::metadata(&cache_file)?;
    let modified = metadata.modified()?;
    let age = SystemTime::now().duration_since(modified)?;

    if age > timeout {
        return Ok(None);
    }

    let bytes = fs::read(&cache_file)?;
    Ok(Some(bytes))
}

async fn get_cached_avatar_fallback(
    url: &str,
    cache_store: &Option<PathBuf>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(None),
    };

    let cache_file = get_cache_file_path(cache_store, url);

    if !cache_file.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&cache_file)?;
    Ok(Some(bytes))
}

async fn save_to_cache(
    url: &str,
    bytes: &[u8],
    cache_store: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(()),
    };

    fs::create_dir_all(cache_store)?;
    let cache_file = get_cache_file_path(cache_store, url);
    fs::write(&cache_file, bytes)?;
    Ok(())
}

fn get_cache_file_path(cache_store: &PathBuf, url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    cache_store.join(format!("avatar_{hash:x}.bin"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_avatar_map_basic_usage() {
        let mut map = ProfileAvatarMap::new();
        let avatar = AvatarData::new("url".to_string(), vec![1, 2, 3], Some("image/png".to_string()));
        map.insert("key", avatar.clone());
        assert_eq!(map.len(), 1);
        assert!(map.get("key").is_some());
        assert_eq!(map.get("key").unwrap().bytes, avatar.bytes);
    }
}
