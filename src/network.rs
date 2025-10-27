//! Network module for fetching and parsing remote org-social files.
//!
//! This module provides functionality to fetch org-social files from remote URLs
//! and parse them into profiles and posts using concurrent HTTP requests.

use crate::profile::Profile;
use crate::post::Post;
use crate::parser::parse_file;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// A set of options for fetching feeds
#[derive(Debug, Clone)]
pub struct FetchOptions {
    // If set, timeout duration for each feed request
    pub fetch_timeout: Option<Duration>,
    // If set, use cached results if available and not older than this duration.
    // If None, all cached results are considered out of date and will be refetched.
    // Cache store has to be set for this to have effect.
    // Uses last modified time of the cache file to determine age.
    pub cache_timeout: Option<Duration>,
    // Optional path to a cache folder. If None, no caching is performed.
    pub cache_store: Option<PathBuf>,
}

impl FetchOptions {
    pub fn new(fetch_timeout: Option<Duration>, cache_timeout: Option<Duration>, cache_store: Option<PathBuf>) -> Self {
        Self {
            fetch_timeout,
            cache_timeout,
            cache_store,
        }
    }
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            fetch_timeout: None,
            cache_timeout: None,
            cache_store: None,
        }
    }
}

/// Fetches and parses org-social files from followed users concurrently.
/// # Arguments
///
/// * `followed_users` - A vector of tuples containing (identifier, url) pairs
/// * `options` - FetchOptions controlling timeout and caching behavior
/// # Returns
///
/// A vector of tuples containing (Profile, Vec\<Post\>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds_with_options(followed_users: Vec<(String, String)>, options: FetchOptions) -> Vec<(Profile, Vec<Post>, String)> {
    let client = std::sync::Arc::new(reqwest::Client::new());
    let options = std::sync::Arc::new(options);

    let fetch_futures: Vec<_> = followed_users
        .into_iter()
        .map(|(identifier, url)| {
            let client = client.clone();
            let identifier = identifier.clone();
            let url = url.clone();
            let options = options.clone();
            
            tokio::spawn(async move {
                match fetch_and_parse_feed_cached(&client, &url, &options).await {
                    Ok((profile, posts)) => Some((profile, posts, url)),
                    Err(e) => {
                        eprintln!("Failed to fetch feed for {identifier} at {url}: {e}");
                        None
                    }
                }
            })
        })
        .collect();

    // Wait for all futures to complete and collect successful results
    let mut results = Vec::new();
    for future in fetch_futures {
        if let Ok(Some((profile, posts, url))) = future.await {
            results.push((profile, posts, url));
        }
    }
    
    results
}

/// Fetches and parses org-social files from followed users concurrently.
/// # Arguments
///
/// * `followed_users` - A vector of tuples containing (identifier, url) pairs
/// * `timeout` - Optional timeout duration for each feed request. If None, no timeout is applied.
/// # Returns
///
/// A vector of tuples containing (Profile, Vec<Post>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds(followed_users: Vec<(String, String)>, timeout: Option<Duration>) -> Vec<(Profile, Vec<Post>, String)> {
    let options = FetchOptions {
        fetch_timeout: timeout,
        cache_timeout: None,
        cache_store: None,
    };
    get_feeds_with_options(followed_users, options).await
}

/// Fetches and parses org-social files from a profile's follow list.
///
/// This is a convenient wrapper around `get_feeds` that extracts the follow list
/// from a profile and fetches all followed users' feeds.
/// # Arguments
/// * `profile` - The profile containing the follow list
/// * `timeout` - Optional timeout duration for each feed request. If None, no timeout is applied.
///
/// # Returns
///
/// A vector of tuples containing (Profile, Vec<Post>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds_from_profile(profile: &Profile, timeout: Option<Duration>) -> Vec<(Profile, Vec<Post>, String)> {
    match profile.follow() {
        Some(followed_users) => {
            get_feeds(followed_users.clone(), timeout).await
        }
        None => Vec::new(),
    }
}

/// Fetches and parses org-social files from a profile's follow list with a default 30-second timeout.
///
/// This is a convenience function that applies a reasonable default timeout.
/// # Arguments
/// * `profile` - The profile containing the follow list
///
/// # Returns
///
/// A vector of tuples containing (Profile, Vec<Post>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds_from_profile_with_timeout(profile: &Profile) -> Vec<(Profile, Vec<Post>, String)> {
    get_feeds_from_profile(profile, Some(Duration::from_secs(30))).await
}

/// Attempts to retrieve a cached feed if available and not expired.
///
/// # Arguments
/// * `url` - The URL of the feed
/// * `cache_timeout` - If set, maximum age of cached content to accept. If None, all cached content is considered out of date.
/// * `cache_store` - Path to the cache directory
///
/// # Returns
/// * `Ok(Some(content))` - Cached content is available and valid
/// * `Ok(None)` - No valid cache available (either doesn't exist, expired, or cache_timeout is None)
/// * `Err(e)` - Error reading cache
pub async fn get_cached_feed(
    url: &str,
    cache_timeout: Option<Duration>,
    cache_store: &Option<PathBuf>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(None),
    };

    let cache_file = get_cache_file_path(cache_store, url);
    
    if !cache_file.exists() {
        return Ok(None);
    }

    // If cache_timeout is None, consider everything out of date
    let timeout = match cache_timeout {
        Some(t) => t,
        None => return Ok(None),
    };

    // Check cache age
    let metadata = fs::metadata(&cache_file)?;
    let modified = metadata.modified()?;
    let age = SystemTime::now().duration_since(modified)?;
    
    if age > timeout {
        return Ok(None);
    }

    // Read and return cached content
    let content = fs::read_to_string(&cache_file)?;
    Ok(Some(content))
}

/// Attempts to retrieve a cached feed regardless of age.
/// This is used as a fallback when network fetching fails.
async fn get_cached_feed_fallback(
    url: &str,
    cache_store: &Option<PathBuf>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(None),
    };

    let cache_file = get_cache_file_path(cache_store, url);
    if !cache_file.exists() {
        return Ok(None);
    }

    // Read and return cached content without checking age
    let content = fs::read_to_string(&cache_file)?;
    Ok(Some(content))
}

/// Saves fetched content to cache.
///
/// # Arguments
/// * `url` - The URL of the feed
/// * `content` - The content to cache
/// * `cache_store` - Path to the cache directory
async fn save_to_cache(
    url: &str,
    content: &str,
    cache_store: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cache_store = match cache_store {
        Some(path) => path,
        None => return Ok(()),
    };

    // Create cache directory if it doesn't exist
    fs::create_dir_all(cache_store)?;

    let cache_file = get_cache_file_path(cache_store, url);
    fs::write(&cache_file, content)?;
    
    Ok(())
}

/// Generates a cache file path based on the URL.
///
/// # Arguments
/// * `cache_store` - Path to the cache directory
/// * `url` - The URL to generate a cache filename for
///
/// # Returns
/// * PathBuf pointing to the cache file
fn get_cache_file_path(cache_store: &PathBuf, url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    
    cache_store.join(format!("feed_{:x}.org", hash))
}

/// Fetches a single org-social file with caching support.
///
/// This function first attempts to retrieve valid cached content based on the cache_timeout setting.
/// If no valid cache is available, it fetches from the network. If network fetching fails,
/// it falls back to using any available cached content regardless of age.
///
/// # Arguments
/// * `client` - The HTTP client to use for the request
/// * `url` - The URL of the org-social file to fetch
/// * `options` - FetchOptions controlling timeout and caching
/// # Returns
///
/// A Result containing a tuple of (Profile, Vec<Post>) on success, or an error on failure
async fn fetch_and_parse_feed_cached(
    client: &reqwest::Client,
    url: &str,
    options: &FetchOptions,
) -> Result<(Profile, Vec<Post>), Box<dyn std::error::Error + Send + Sync>> {
    // Try to get from cache first
    if let Ok(Some(content)) = get_cached_feed(url, options.cache_timeout, &options.cache_store).await {
        let (profile, posts) = parse_file(&content, Some(url.to_string()));
        return Ok((profile, posts));
    }

    // Fetch from network
    let request_future = async {
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error {}: {}", response.status(), url).into());
        }
        
        let content = response.text().await?;

        // Save to cache (ignore errors, do in the background)
        let url_owned = url.to_string();
        let cache_store_owned = options.cache_store.clone();

        let (profile, posts) = parse_file(&content, Some(url.to_string()));

        // Save the new cached file in the background
        tokio::spawn(async move {
            let _ = save_to_cache(&url_owned, &content, &cache_store_owned).await;
        });

        Ok((profile, posts))
    };

    let fetch_result = match options.fetch_timeout {
        Some(duration) => {
            match tokio::time::timeout(duration, request_future).await {
                Ok(result) => result,
                Err(_) => Err(format!("Timeout after {duration:?} while fetching {url}").into()),
            }
        }
        None => request_future.await,
    };

    // If fetching failed, try to use cached content as fallback (regardless of age)
    match fetch_result {
        Ok(result) => Ok(result),
        Err(e) => {
            if let Ok(Some(content)) = get_cached_feed_fallback(url, &options.cache_store).await {
                let (profile, posts) = parse_file(&content, Some(url.to_string()));
                Ok((profile, posts))
            } else {
                Err(e)
            }
        }
    }
}

/// Fetches and parses org-social files from followed users concurrently with a default 30-second timeout.
/// 
/// This is a convenience function that applies a reasonable default timeout.
/// # Arguments
///
/// * `followed_users` - A vector of tuples containing (identifier, url) pairs
/// # Returns
///
/// A vector of tuples containing (Profile, Vec\<Post\>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds_with_timeout(followed_users: Vec<(String, String)>) -> Vec<(Profile, Vec<Post>, String)> {
    get_feeds(followed_users, Some(Duration::from_secs(30))).await
}

/// Clears the cached feed for a specific URL.
///
/// # Arguments
/// * `url` - The URL of the feed to clear from cache
/// * `cache_store` - Path to the cache directory
///
/// # Returns
/// Ok(true) if cache file was deleted, Ok(false) if it didn't exist, Err on failure
pub fn clear_cached_feed(url: &str, cache_store: &PathBuf) -> Result<bool, Box<dyn std::error::Error>> {
    let cache_file = get_cache_file_path(cache_store, url);
    
    if cache_file.exists() {
        fs::remove_file(&cache_file)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Clears all cached feeds in the cache directory.
///
/// # Arguments
/// * `cache_store` - Path to the cache directory
///
/// # Returns
/// Ok(count) with the number of files deleted, or Err on failure
pub fn flush_cache(cache_store: &PathBuf) -> Result<usize, Box<dyn std::error::Error>> {
    if !cache_store.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(cache_store)? {
        let entry = entry?;
        let path = entry.path();
        
        // Only delete files that match our cache naming pattern
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("feed_") && filename.ends_with(".org") {
                    fs::remove_file(&path)?;
                    count += 1;
                }
            }
        }
    }
    
    Ok(count)
}