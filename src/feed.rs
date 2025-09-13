//! Feed module for managing collections of social media posts.
//!
//! This module provides functionality to create, filter, and display
//! feeds of org-social posts from multiple sources.
//! The feed represantation is by default sorted chronologically with newest posts first.

use std::collections::HashMap;
use std::sync::Arc;

use crate::profile::Profile;
use crate::post::Post;
use crate::network;
use chrono::{DateTime, FixedOffset};

/// Represents a collection of org-social posts from various sources.
///
/// A feed contains posts, associated profiles, and a mapping from post ids to their author's profiles.
pub struct Feed {
    pub posts: Vec<Post>,
    pub profiles: Vec<Arc<Profile>>,
    pub profile_map: HashMap<String, Arc<Profile>>, // Maps post ID to profile
}

impl Feed {
    pub async fn new(user_profile: &Profile, user_posts: Vec<Post>) -> Result<Feed, Box<dyn std::error::Error>>{
        let mut all_posts = Vec::new();
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
            all_posts.push(post);
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
                all_posts.push(post);
            }
        }

        // Build post->profile map using Arc<Profile>
        let mut profile_map: HashMap<String, Arc<Profile>> = HashMap::new();
        for post in &all_posts {
            let profile_arc = profiles.iter()
                .find(|p| post.author().as_deref() == Some(p.nick()))
                .cloned()
                .unwrap_or(user_profile_arc.clone());
            profile_map.insert(post.id().to_string(), profile_arc);
        }

        Ok(Feed { posts: all_posts, profiles, profile_map })
    }

    /// Creates a feed with the posts sorted chronologically, newest first.
    pub async fn create_combined_feed(
        user_profile: &Profile,
        user_posts: Vec<Post>,
    ) -> Result<Feed, Box<dyn std::error::Error>> {
        let mut feed = Self::new(user_profile, user_posts).await?;

        // Sort posts chronologically (newest first)
        feed.posts.sort_by(|a, b| {
            match (a.time(), b.time()) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse order for newest first
                (Some(_), None) => std::cmp::Ordering::Less,         // Posts with time come before posts without
                (None, Some(_)) => std::cmp::Ordering::Greater,      // Posts without time come after posts with time
                (None, None) => std::cmp::Ordering::Equal,           // Equal if both don't have time
            }
        });

        Ok(feed)
    }
    
    pub fn create_user_feed(user_profile: &Profile, user_posts: Vec<Post>) -> Feed {
        let mut posts = Vec::new();
        let mut profiles: Vec<Arc<Profile>> = Vec::new();
        let user_profile_arc = Arc::new(user_profile.clone());
        profiles.push(user_profile_arc.clone());

        // Set author for user's own posts
        for mut post in user_posts {
            post.set_author(user_profile.nick().to_string());
            posts.push(post);
        }

        // Sort posts chronologically (newest first)
        posts.sort_by(|a, b| {
            match (a.time(), b.time()) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse order for newest first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        // Build post->profile map using Arc<Profile>
        let mut profile_map: HashMap<String, Arc<Profile>> = HashMap::new();
        for post in &posts {
            let profile_arc = profiles.iter()
                .find(|p| post.author().as_deref() == Some(p.nick()))
                .cloned()
                .unwrap_or(user_profile_arc.clone());
            profile_map.insert(post.id().to_string(), profile_arc);
        }

        Feed { posts, profiles, profile_map }
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
    /// A vector of references to posts within the time range.threading
    pub fn posts_in_range(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Vec<&Post> {
        self.posts
            .iter()
            .filter(|post| {
                if let Some(post_time) = post.time() {
                    post_time >= start && post_time <= end
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn get_recent_posts(&self, limit: usize) -> Vec<&Post> {
        self.posts.iter().take(limit).collect()
    }

    pub fn posts_from_source(&self, source: &str) -> Vec<&Post> {
        self.posts
            .iter()
            .filter(|post| {
                post.source()
                    .as_ref()
                    .map(|s| s == source)
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn sources(&self) -> Vec<String> {
        let mut sources: Vec<String> = self.posts
            .iter()
            .filter_map(|post| post.source().as_ref())
            .cloned()
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

    pub fn profile_for_post(&self, post: &Post) -> Option<&Arc<Profile>> {
        self.profile_map.get(post.id())
    }

}

impl std::fmt::Display for Feed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Feed with {} posts:", self.posts.len())?;
        for (i, post) in self.posts.iter().enumerate() {
            writeln!(f, "--- Post {} ---", i + 1)?;
            if let Some(time) = post.time() {
                writeln!(f, "Time: {time}")?;
            }
            if let Some(source) = post.source() {
                writeln!(f, "Source: {source}")?;
            }
            writeln!(f, "{post}")?;
            writeln!(f)?;
        }
        Ok(())
    }
}
