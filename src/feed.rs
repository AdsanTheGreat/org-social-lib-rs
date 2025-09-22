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
use crate::network;

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
        view.borrow_mut().update_content(&self);
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
        v.update_content(&self);
        v.refresh();
    }

    /// Update all views with current data.
    pub fn update_all_views(&self) {
        for view in &self.views {
            let mut v = view.borrow_mut();
            v.update_content(&self);
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
}

/// A simple chronologically sorted feed view.
///
/// This is the basic feed implementation that presents posts in chronological order.
/// It operates on the underlying Feed data without duplicating storage.
pub struct SimpleFeed {
    posts: Vec<Rc<RefCell<Post>>>,
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
