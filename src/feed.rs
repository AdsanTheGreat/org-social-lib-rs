//! Feed module for managing collections of social media posts.
//!
//! This module provides functionality to create, filter, and display
//! feeds of org-social posts from multiple sources.
//! 
//! The main `Feed` struct serves as a storage and management layer for posts and profiles,
//! while delegating presentation and filtering to various `FeedView` implementations.

use std::collections::HashMap;
use std::sync::Arc;

use crate::profile::Profile;
use crate::post::Post;
use std::rc::Rc;
use std::cell::RefCell;
use crate::feed_view::FeedView;
#[cfg(feature = "fetch")]
use crate::network;

#[cfg(all(feature = "fetch", feature = "relay"))]
/// Configuration for relay-based feed creation.
#[derive(Debug, Clone)]
pub struct RelayFeedOptions {
    /// Maximum number of posts to fetch replies for (most recent posts).
    pub max_posts_with_replies: Option<usize>,
    /// Whether to fetch the full reply tree or just direct replies.
    pub fetch_full_tree: bool,
}

#[cfg(all(feature = "fetch", feature = "relay"))]
impl Default for RelayFeedOptions {
    fn default() -> Self {
        Self {
            max_posts_with_replies: Some(20),
            fetch_full_tree: true,
        }
    }
}

/// Main feed storage that manages posts, profiles, and multiple views.
///
/// The Feed struct serves as the central storage for posts and profiles,
/// and manages multiple views that can present the data in different ways.
/// Views share immutable references to the underlying data without cloning.
pub struct Feed {
    pub posts: Vec<Rc<RefCell<Post>>>,
    pub profiles: Vec<Arc<Profile>>,
    pub profile_map: HashMap<String, Arc<Profile>>, // Maps post ID to profile
    pub views: Vec<Rc<RefCell<dyn FeedView>>>,
}

impl Feed {
    /// Add a view to this feed.
    pub fn add_view(&mut self, view: Rc<RefCell<dyn FeedView>>) {
        // Update the view with current data
        view.borrow_mut().update_content(self);
        self.views.push(view);
    }

    /// Remove a view from this feed.
    pub fn remove_view(&mut self, view_name: &str) -> bool {
        let initial_len = self.views.len();
        self.views.retain(|view| {
            view.borrow().view_name() != view_name
        });
        self.views.len() != initial_len
    }

    /// Add posts to the feed and update all views.
    pub fn add_posts(&mut self, new_posts: Vec<Post>) {
    let rc_posts: Vec<Rc<RefCell<Post>>> = new_posts.into_iter().map(|p| Rc::new(RefCell::new(p))).collect();
    self.posts.extend(rc_posts);
        self.update_all_views();
    }

    /// Add a single post to the feed and update all views.
    pub fn add_post(&mut self, post: Post) -> Rc<RefCell<Post>> {
        let rc_post = Rc::new(RefCell::new(post));
        self.posts.push(rc_post.clone());
        self.update_all_views();
        rc_post
    }

    /// Add a profile to the feed.
    pub fn add_profile(&mut self, profile: Profile) -> Arc<Profile> {
        let profile_arc = Arc::new(profile);
        self.profiles.push(profile_arc.clone());
        profile_arc
    }

    /// Set the author for posts and update the profile map.
    pub fn set_posts_author(&mut self, posts_ids: &[String], author_nick: &str) {
        if let Some(profile_arc) = self.profiles.iter().find(|p| p.nick() == author_nick).cloned() {
            for post_id in posts_ids {
                self.profile_map.insert(post_id.clone(), profile_arc.clone());
            }
        }
    }

    pub fn update_view(&self, view: &Rc<RefCell<dyn FeedView>>) {
        let mut v = view.borrow_mut();
        v.update_content(self);
        v.refresh();
    }

    /// Update all views with current data.
    pub fn update_all_views(&self) {
        for view in &self.views {
            let mut v = view.borrow_mut();
            v.update_content(self);
            v.refresh();
        }
    }

    /// Get the profile for a specific post.
    pub fn profile_for_post(&self, post: &Rc<RefCell<Post>>) -> Option<&Arc<Profile>> {
        self.profile_map.get(post.borrow().id())
    }

    /// Get all posts from a single profile.
    pub fn posts_from_profile(&self, profile: &Profile) -> Vec<Rc<RefCell<Post>>> {
        self.posts.iter().filter(|post| {
            self.profile_map.get(post.borrow().id()).map(|p| p.as_ref() == profile).unwrap_or(false)
        }).cloned().collect()
    }

    /// Get the number of posts.
    pub fn len(&self) -> usize {
        self.posts.len()
    }

    /// Check if the feed is empty.
    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }

    /// Create a feed from posts for testing purposes.
    pub fn from_posts(posts: Vec<Post>) -> Self {
        let rc_posts: Vec<Rc<RefCell<Post>>> = posts.into_iter().map(|p| Rc::new(RefCell::new(p))).collect();
        Feed {
            posts: rc_posts,
            profiles: Vec::new(),
            profile_map: HashMap::new(),
            views: Vec::new(),
        }
    }

    /// Apply a language filter to all the views.
    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    /// Strips all filters before applying the new one.
    pub fn filter_by_lang(&self, lang: &str) {
        self.update_all_views();
        for view in &self.views {
            let lang = lang.to_string();
            let closure = Box::new(move |post: &Post| post.lang().as_deref() == Some(&lang));
            let mut v = view.borrow_mut();
            v.apply_filter(closure);
        }
    }

    /// Apply a tag filter to all the views. At least one tag must match.
    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    /// Strips all filters before applying the new one.
    pub fn filter_by_tag(&self, tag: &str) {
        self.update_all_views();
        for view in &self.views {
            let tag = tag.to_string();
            let closure = Box::new(move |post: &Post| {
                post.tags()
                    .as_ref()
                    .map(|tags| tags.iter().any(|t| t == &tag))
                    .unwrap_or(false)
            });
            let mut v = view.borrow_mut();
            v.apply_filter(closure);
        }
    }

    /// Apply an author filter to all the views.
    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    /// Strips all filters before applying the new one.
    pub fn filter_by_author(&self, author: &str) {
        self.update_all_views();
        for view in &self.views {
            let author = author.to_string();
            let closure = Box::new(move |post: &Post| post.author().as_deref() == Some(&author));
            let mut v = view.borrow_mut();
            v.apply_filter(closure);
        }
    }

    /// Apply a source filter to all the views.
    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    /// Strips all filters before applying the new one.
    pub fn filter_by_source(&self, source: &str) {
        self.update_all_views();
        for view in &self.views {
            let source = source.to_string();
            let closure = Box::new(move |post: &Post| {
                post.source()
                    .as_ref()
                    .map(|s| s == &source)
                    .unwrap_or(false)
            });
            let mut v = view.borrow_mut();
            v.apply_filter(closure);
        }
    }

    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    /// Strips all filters before applying the new one.
    pub fn filter_by_group(&self, group: &str) {
        self.update_all_views();
        for view in &self.views {
            let group = group.to_string();
            let closure = Box::new(move |post: &Post| {
                post.group().as_ref().map(|(g, _)| g == &group).unwrap_or(false)
            });
            let mut v = view.borrow_mut();
            v.apply_filter(closure);
        }
    }

    /// Apply a custom filter closure to the posts in all views.
    /// This keeps the feed itself unchanged.
    /// The views can be updated to show all posts again.
    pub fn filter_custom(&self, filter_fn: Box<dyn Fn(&Post) -> bool + Send + Sync + 'static>) {
        use std::sync::Arc;
        self.update_all_views();
        let filter_fn = Arc::new(filter_fn);
        for view in &self.views {
            let filter_fn = Arc::clone(&filter_fn);
            let mut v = view.borrow_mut();
            v.apply_filter(Box::new(move |post| filter_fn(post)));
        }
    }

    #[cfg(feature = "fetch")]
    /// Create a new Feed from user profile and posts, fetching followed feeds.
    pub async fn new_from_user(user_profile: &Profile, user_posts: Vec<Post>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut all_posts: Vec<Rc<RefCell<Post>>> = Vec::new();
        let mut profiles: Vec<Arc<Profile>> = Vec::new();

        // Add user profile to profiles
        let user_profile_arc = Arc::new(user_profile.clone());
        profiles.push(user_profile_arc.clone());

        // Fetch posts from followed users and collect profiles
        let followed_feeds = network::get_feeds_from_profile_with_timeout(user_profile).await;
        for (profile, _, _) in &followed_feeds {
            profiles.push(Arc::new(profile.clone()));
        }

        // Build post list
        for mut post in user_posts {
            post.set_author(user_profile.nick().to_string());
            all_posts.push(Rc::new(RefCell::new(post)));
        }
        for (profile, posts, source) in followed_feeds {
            let author_nick = if profile.nick().is_empty() {
                "unknown".to_string()
            } else {
                profile.nick().to_string()
            };
            for mut post in posts {
                post.set_author(author_nick.clone());
                post.set_source(Some(source.clone()));
                all_posts.push(Rc::new(RefCell::new(post)));
            }
        }

        // Build post->profile map using Arc<Profile>
        let mut profile_map: HashMap<String, Arc<Profile>> = HashMap::new();
        for post in &all_posts {
            let profile_arc = profiles.iter()
                .find(|p| post.borrow().author().as_deref() == Some(p.nick()))
                .cloned()
                .unwrap_or(user_profile_arc.clone());
            profile_map.insert(post.borrow().id().to_string(), profile_arc);
        }

        Ok(Feed { posts: all_posts, profiles, profile_map, views: Vec::new() })
    }

    #[cfg(not(feature = "fetch"))]
    /// Create a new Feed from user profile and posts without fetching remote feeds.
    pub async fn new_from_user(user_profile: &Profile, user_posts: Vec<Post>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::from_user_posts(user_profile, user_posts))
    }

    /// Create a Feed with user posts only (no network fetching).
    pub fn from_user_posts(user_profile: &Profile, user_posts: Vec<Post>) -> Self {
        let mut posts: Vec<Rc<RefCell<Post>>> = Vec::new();
        let mut profiles: Vec<Arc<Profile>> = Vec::new();
        let user_profile_arc = Arc::new(user_profile.clone());
        profiles.push(user_profile_arc.clone());

        // Set author for user's own posts
        for mut post in user_posts {
            post.set_author(user_profile.nick().to_string());
            posts.push(Rc::new(RefCell::new(post)));
        }

        // Build post->profile map using Arc<Profile>
        let mut profile_map: HashMap<String, Arc<Profile>> = HashMap::new();
        for post in &posts {
            let profile_arc = profiles.iter()
                .find(|p| post.borrow().author().as_deref() == Some(p.nick()))
                .cloned()
                .unwrap_or(user_profile_arc.clone());
            profile_map.insert(post.borrow().id().to_string(), profile_arc);
        }

        Feed { posts, profiles, profile_map, views: Vec::new() }
    }

    /// Create a Feed from posts and profiles.
    /// Assumes that authors are set correctly in posts
    pub fn from_posts_and_profiles(posts: Vec<Post>, profiles: Vec<Profile>) -> Self {
        let rc_posts: Vec<Rc<RefCell<Post>>> = posts.into_iter().map(|p| Rc::new(RefCell::new(p))).collect();
        let arc_profiles: Vec<Arc<Profile>> = profiles.into_iter().map(Arc::new).collect();

        // Build post->profile map using Arc<Profile>
        let mut profile_map: HashMap<String, Arc<Profile>> = HashMap::new();
        for post in &rc_posts {
            if let Some(author) = post.borrow().author() {
                if let Some(profile_arc) = arc_profiles.iter().find(|p| p.nick() == author).cloned() {
                    profile_map.insert(post.borrow().id().to_string(), profile_arc);
                }
            }
        }

        Feed { posts: rc_posts, profiles: arc_profiles, profile_map, views: Vec::new() }
    }

    #[cfg(all(feature = "fetch", feature = "relay"))]
    pub async fn new_threaded_from_relay(
        user_profile: &Profile,
        user_posts: Vec<Post>,
        relay_client: &crate::relay::RelayClient,
        fetch_options: Option<network::FetchOptions>,
        relay_options: Option<RelayFeedOptions>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use std::collections::HashSet;

        let fetch_options = fetch_options.unwrap_or_default();
        let relay_options = relay_options.unwrap_or_default();
        let mut all_posts: Vec<Post> = Vec::new();
        let mut all_profiles: Vec<Profile> = Vec::new();
        let mut seen_profiles: HashSet<String> = HashSet::new();

        // Add user profile
        let user_nick = user_profile.nick().to_string();
        all_profiles.push(user_profile.clone());
        seen_profiles.insert(user_nick.clone());

        // Add user's own posts
        for mut post in user_posts {
            post.set_author(user_nick.clone());
            all_posts.push(post);
        }

        // Fetch posts from followed users
        let followed_list = match user_profile.follow() {
            Some(follows) => follows.clone(),
            None => Vec::new(),
        };
        let followed_feeds = network::get_feeds_with_options(
            followed_list,
            fetch_options.clone()
        ).await;

        // Collect all posts and profiles from followed feeds
        let mut post_urls: Vec<String> = Vec::new();
        for (profile, posts, source) in followed_feeds {
            let author_nick = if profile.nick().is_empty() {
                "unknown".to_string()
            } else {
                profile.nick().to_string()
            };

            // Add profile if not seen
            if seen_profiles.insert(author_nick.clone()) {
                all_profiles.push(profile.clone());
            }

            // Add posts and collect URLs for reply fetching
            for mut post in posts {
                post.set_author(author_nick.clone());
                post.set_source(Some(source.clone()));
                
                // Build post URL for relay query (format: feed_url#post_id)
                // Only include the # if we have a post ID
                let post_id = post.id();
                if !post_id.is_empty() {
                    let post_url = format!("{}#{}", source, post_id);
                    post_urls.push(post_url);
                }
                
                all_posts.push(post);
            }
        }

        // Limit the number of posts we fetch replies for (most recent ones)
        // This prevents fetching replies for hundreds of old posts
        if let Some(max_posts) = relay_options.max_posts_with_replies {
            if post_urls.len() > max_posts {
                // Sort by timestamp (most recent first) if possible
                // For now, just take the last N posts (most recently added)
                post_urls = post_urls.into_iter()
                    .rev()
                    .take(max_posts)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
            }
        }

        println!("Fetching replies for {} posts from relay...", post_urls.len());

        // Fetch replies for all posts using provided relay client
        let mut seen_post_ids: HashSet<String> = all_posts.iter()
            .map(|p| p.id().to_string())
            .collect();

        // Fetch all replies concurrently to improve performance
        let reply_futures: Vec<_> = post_urls.iter()
            .map(|post_url| {
                let relay = relay_client.clone();
                let url = post_url.clone();
                tokio::spawn(async move {
                    (url.clone(), relay.get_replies(&url).await)
                })
            })
            .collect();

        // Collect all unique reply URLs from all relay responses
        let mut all_reply_urls = HashSet::new();
        for future in reply_futures {
            if let Ok((post_url, Ok(replies_response))) = future.await {
                if !replies_response.data.is_empty() {
                    println!("Got {} reply trees for {}", replies_response.data.len(), post_url);
                }
                
                // Collect reply URLs from the tree structure
                let mut reply_urls = Vec::new();
                if relay_options.fetch_full_tree {
                    // Recursively collect all replies in the tree
                    collect_reply_urls_from_response(&replies_response.data, &mut reply_urls);
                } else {
                    // Only collect direct replies (first level)
                    collect_direct_reply_urls(&replies_response.data, &mut reply_urls);
                }
                all_reply_urls.extend(reply_urls);
            }
        }

        println!("Found {} unique reply URLs to fetch", all_reply_urls.len());

        // Resolve all unique reply URLs in one batch
        if !all_reply_urls.is_empty() {
            let reply_urls_vec: Vec<String> = all_reply_urls.into_iter().collect();
            match resolve_reply_posts(&reply_urls_vec, &fetch_options).await {
                Ok((mut reply_posts, reply_profiles)) => {
                    // Add new profiles
                    for profile in reply_profiles {
                        let nick = profile.nick().to_string();
                        if seen_profiles.insert(nick) {
                            all_profiles.push(profile);
                        }
                    }
                    
                    // Add new posts (avoid duplicates)
                    for post in reply_posts.drain(..) {
                        if seen_post_ids.insert(post.id().to_string()) {
                            all_posts.push(post);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to resolve reply posts: {}", e);
                }
            }
        }

        // Build feed with all collected posts and profiles
        Ok(Self::from_posts_and_profiles(all_posts, all_profiles))
    }
}

#[cfg(all(feature = "fetch", feature = "relay"))]
/// Helper function to recursively collect reply URLs from relay response tree.
fn collect_reply_urls_from_response(nodes: &[crate::relay::ReplyNode], acc: &mut Vec<String>) {
    for node in nodes {
        acc.push(node.post.clone());
        if !node.children.is_empty() {
            collect_reply_urls_from_response(&node.children, acc);
        }
    }
}

#[cfg(all(feature = "fetch", feature = "relay"))]
/// Helper function to collect only direct reply URLs (first level, not recursive).
fn collect_direct_reply_urls(nodes: &[crate::relay::ReplyNode], acc: &mut Vec<String>) {
    for node in nodes {
        acc.push(node.post.clone());
        // Don't recurse into children - only get first level
    }
}

#[cfg(all(feature = "fetch", feature = "relay"))]
/// Helper function to resolve reply post URLs into actual posts and profiles.
async fn resolve_reply_posts(
    post_urls: &[String],
    fetch_options: &network::FetchOptions,
) -> Result<(Vec<Post>, Vec<Profile>), Box<dyn std::error::Error>> {
    use std::collections::{HashMap, HashSet};

    // Group post URLs by feed
    let mut feed_requests: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for url in post_urls {
        if let Some((feed, fragment)) = url.split_once('#') {
            let feed = feed.trim();
            let fragment = fragment.trim();
            if !feed.is_empty() && !fragment.is_empty() {
                feed_requests.entry(feed.to_string())
                    .or_default()
                    .push((fragment.to_string(), url.clone()));
            }
        }
    }

    let fetch_targets: Vec<(String, String)> = feed_requests
        .keys()
        .map(|feed| (feed.clone(), feed.clone()))
        .collect();

    // Fetch all feeds (with caching)
    let fetched = network::get_feeds_with_options(fetch_targets, fetch_options.clone()).await;
    
    let mut resolved_posts = Vec::new();
    let mut resolved_profiles = Vec::new();
    let mut seen_profiles: HashSet<String> = HashSet::new();

    for (profile, posts, source_url) in fetched {
        if let Some(requested_posts) = feed_requests.get(&source_url) {
            let mut posts_by_id: HashMap<String, Post> = posts
                .into_iter()
                .map(|mut post| {
                    if post.author().is_none() {
                        post.set_author(profile.nick().to_string());
                    }
                    (post.id().to_string(), post)
                })
                .collect();

            // Add profile if not seen
            let profile_key = profile.nick().to_string();
            if seen_profiles.insert(profile_key) {
                resolved_profiles.push(profile.clone());
            }

            // Extract requested posts
            for (post_id, _) in requested_posts {
                if let Some(post) = posts_by_id.remove(post_id) {
                    resolved_posts.push(post);
                }
            }
        }
    }

    Ok((resolved_posts, resolved_profiles))
}

/// A simple chronologically sorted feed view.
///
/// This is the basic feed implementation that presents posts in chronological order.
/// It operates on the underlying Feed data without duplicating storage.
pub struct SimpleFeed {
    pub posts: Vec<Rc<RefCell<Post>>>,
}

impl Default for SimpleFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleFeed {
    /// Create a new empty SimpleFeed.
    pub fn new() -> Self {
        SimpleFeed {
            posts: Vec::new(),
        }
    }

    /// Create a new SimpleFeed from existing data.
    pub fn from_feed(feed: &Feed) -> Self {
        let mut sorted_posts = feed.posts.to_vec();

        // Sort posts chronologically (newest first)
        sorted_posts.sort_by(|a, b| {
            match (a.borrow().time(), b.borrow().time()) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse order for newest first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        SimpleFeed {
            posts: sorted_posts,
        }
    }

    /// Filter posts by a specific time range.
    ///
    /// Returns posts that fall within the specified start and end times.
    ///
    /// # Arguments
    ///
    /// * `start` - The start time of the range (inclusive)
    /// * `end` - The end time of the range (inclusive)
    ///
    /// # Returns
    ///
    /// A vector of references to posts within the time range.
    pub fn posts_in_range(
        &self,
        start: chrono::DateTime<chrono::FixedOffset>,
        end: chrono::DateTime<chrono::FixedOffset>,
    ) -> Vec<Rc<RefCell<Post>>> {
        self.posts
            .iter()
            .filter(|post| {
                if let Some(post_time) = post.borrow().time() {
                    post_time >= start && post_time <= end
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    pub fn get_recent_posts(&self, limit: usize) -> Vec<Rc<RefCell<Post>>> {
        self.posts.iter().take(limit).cloned().collect()
    }

    pub fn posts_from_source(&self, source: &str) -> Vec<Rc<RefCell<Post>>> {
        self.posts
            .iter()
            .filter(|post| {
                post.borrow().source()
                    .as_ref()
                    .map(|s| s == source)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub fn sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self.posts
            .iter()
            .filter_map(|post| post.borrow().source().as_ref().cloned())
            .collect();
        sources.sort();
        sources.dedup();
        sources
    }

    pub fn len(&self) -> usize {
        self.posts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }


}

impl FeedView for SimpleFeed {    
    fn update_content(&mut self, feed: &Feed) {
        self.posts = feed.posts.to_vec();
        self.refresh();
    }
    
    fn view_name(&self) -> &str {
        "Chronological Feed"
    }

    fn len(&self) -> usize {
        self.posts.len()
    }
    
    fn refresh(&mut self) {
        // Sort posts chronologically (newest first)
        self.posts.sort_by(|a, b| {
            match (a.borrow().time(), b.borrow().time()) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse order for newest first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    fn apply_filter(&mut self, filter_fn: Box<dyn Fn(&crate::post::Post) -> bool>) {
        self.posts = self.posts
            .iter()
            .filter(|post| filter_fn(&post.borrow()))
            .cloned()
            .collect();
        self.refresh();
    }
}

impl std::fmt::Display for SimpleFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SimpleFeed with {} posts:", self.posts.len())?;
        for (i, post) in self.posts.iter().enumerate() {
            let post_ref = post.borrow();
            writeln!(f, "--- Post {} ---", i + 1)?;
            if let Some(time) = post_ref.time() {
                writeln!(f, "Time: {time}")?;
            }
            if let Some(source) = post_ref.source() {
                writeln!(f, "Source: {source}")?;
            }
            writeln!(f, "{post_ref}")?;
            writeln!(f)?;
        }
        Ok(())
    }
}