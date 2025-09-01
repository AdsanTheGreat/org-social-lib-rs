//! Network module for fetching and parsing remote org-social files.
//!
//! This module provides functionality to fetch org-social files from remote URLs
//! and parse them into profiles and posts using concurrent HTTP requests.

use crate::profile::Profile;
use crate::post::Post;
use crate::parser::parse_file;
use std::time::Duration;

/// Fetches and parses org-social files from followed users concurrently.
/// # Arguments
///
/// * `followed_users` - A vector of tuples containing (identifier, url) pairs
/// * `timeout` - Optional timeout duration for each feed request. If None, no timeout is applied.
/// # Returns
///
/// A vector of tuples containing (Profile, Vec\<Post\>, String), where the String is the URL, for successfully fetched feeds
pub async fn get_feeds(followed_users: Vec<(String, String)>, timeout: Option<Duration>) -> Vec<(Profile, Vec<Post>, String)> {
    let client = std::sync::Arc::new(reqwest::Client::new());

    let fetch_futures: Vec<_> = followed_users
        .into_iter()
        .map(|(identifier, url)| {
            let client = client.clone();
            let identifier = identifier.clone();
            let url = url.clone();
            let timeout = timeout;
            
            tokio::spawn(async move {
                match fetch_and_parse_feed(&client, &url, timeout).await {
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

/// Fetches a single org-social file and parses it.
///
/// # Arguments
/// * `client` - The HTTP client to use for the request
/// * `url` - The URL of the org-social file to fetch
/// * `timeout` - Optional timeout duration for the request. If None, no timeout is applied.
/// # Returns
///
/// A Result containing a tuple of (Profile, Vec<Post>) on success, or an error on failure
async fn fetch_and_parse_feed(
    client: &reqwest::Client,
    url: &str,
    timeout: Option<Duration>,
) -> Result<(Profile, Vec<Post>), Box<dyn std::error::Error>> {
    let request_future = async {
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error {}: {}", response.status(), url).into());
        }
        
        let content = response.text().await?;
        let (profile, posts) = parse_file(&content, Some(url.to_string()));
        
        Ok((profile, posts))
    };

    match timeout {
        Some(duration) => {
            match tokio::time::timeout(duration, request_future).await {
                Ok(result) => result,
                Err(_) => Err(format!("Timeout after {:?} while fetching {}", duration, url).into()),
            }
        }
        None => request_future.await,
    }
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
