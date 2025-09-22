//! Notifications module for managing user notifications.
//!
//! This module provides functionality to identify and aggregate notifications
//! for a user, including mentions and replies to their posts.
//! Notifications are sorted chronologically with newest first. 
//! Duplicates of the same post are dropped.

use crate::profile::Profile;
use crate::post::Post;
use crate::feed::Feed;
use crate::tokenizer::Token;
use chrono::{DateTime, FixedOffset};
use std::rc::Rc;
use std::cell::RefCell;

/// Types of notifications that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationType {
    /// A post that mentions the user
    Mention,
    /// A reply to one of the user's posts
    Reply,
    /// A post that both mentions the user and replies to their post
    MentionAndReply,
}

/// Represents a notification containing a post and the reason for notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub post: Rc<RefCell<Post>>,
    pub notification_type: NotificationType,
}

impl Notification {
    pub fn new(post: Rc<RefCell<Post>>, notification_type: NotificationType) -> Self {
        Self {
            post,
            notification_type,
        }
    }
}

/// Represents a collection of notifications for a user
pub struct NotificationFeed {
    pub notifications: Vec<Notification>,
    target_profile: Profile,
}

impl NotificationFeed {
    /// Create a notification feed for a user based on their profile, using Feed as base
    pub fn from_feed(feed: &Feed, user_profile: &Profile) -> NotificationFeed {
        let mut notifications = Vec::new();
        let user_posts = feed.posts_from_profile(&user_profile);

        for post in feed.posts.iter() {
            let post_ref = post.borrow();
            // Skip the user's own posts
            if post_ref.author() == &Some(user_profile.nick().to_string()) {
                continue;
            }

            let is_mention = is_mention_to_user(&post_ref, &user_profile);
            let is_reply = is_reply_to_user(&post_ref, &user_posts);

            let notification_type = match (is_mention, is_reply) {
                (true, true) => Some(NotificationType::MentionAndReply),
                (true, false) => Some(NotificationType::Mention),
                (false, true) => Some(NotificationType::Reply),
                (false, false) => None,
            };

            if let Some(notification_type) = notification_type {
                notifications.push(Notification::new(post.clone(), notification_type));
            }
        }

        // Sort notifications chronologically (newest first)
        notifications.sort_by(|a, b| {
            let a_time = a.post.borrow().time();
            let b_time = b.post.borrow().time();
            match (a_time, b_time) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse order for newest first
                (Some(_), None) => std::cmp::Ordering::Less,         // Posts with time come before posts without
                (None, Some(_)) => std::cmp::Ordering::Greater,      // Posts without time come after posts with time
                (None, None) => std::cmp::Ordering::Equal,           // Equal if both don't have time
            }
        });

        NotificationFeed { notifications, target_profile: user_profile.clone() }
    }

    /// Filter notifications by a specific time range
    ///
    /// # Arguments
    ///
    /// * `start` - The start time of the range (inclusive)
    /// * `end` - The end time of the range (inclusive)
    ///
    /// # Returns
    ///
    /// A vector of references to notifications within the time range
    pub fn notifications_in_range(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|notification| {
                if let Some(post_time) = notification.post.borrow().time() {
                    post_time >= start && post_time <= end
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get the most recent notifications up to a specified limit
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of notifications to return
    ///
    /// # Returns
    ///
    /// A vector of references to the most recent notifications
    pub fn get_recent_notifications(&self, limit: usize) -> Vec<&Notification> {
        self.notifications.iter().take(limit).collect()
    }

    /// Filter notifications by type
    ///
    /// # Arguments
    ///
    /// * `notification_type` - The type of notifications to filter for
    ///
    /// # Returns
    ///
    /// A vector of references to notifications of the specified type
    pub fn notifications_by_type(&self, notification_type: NotificationType) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|notification| notification.notification_type == notification_type)
            .collect()
    }

    /// Get the total number of notifications
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    /// Check if the notification feed is empty
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}

/// Check if a post mentions a specific user
///
/// This function examines the post's tokens to find mention tokens that reference
/// the user's profile URL or username, or if it's content contains the username with an "@" prefix.
///
/// # Arguments
///
/// * `post` - The post to check for mentions
/// * `user_profile` - The user's profile to check for mentions of
///
/// # Returns
///
/// `true` if the post mentions the user, `false` otherwise
fn is_mention_to_user(post: &Post, user_profile: &Profile) -> bool {
    let user_nick = user_profile.nick();
    let user_source = user_profile.source();

    for token in post.tokens() {
        if let Token::Mention { url, username } = token {
            // Sometimes, users are mentioned with an "@" prefix
            if username == user_nick || format!("@{username}") == user_nick {
                return true;
            }
            
            // Check if the mention URL matches the user's source URL
            if let Some(source) = user_source {
                if url == source {
                    return true;
                }
            }

            // As fallback - check if content contains @username - mentions without org-mode links
            if post.content().contains(&format!("@{username}")) {
                return true;
            }
        }
    }

    false
}

/// Check if a post is a reply to any of the user's posts
///
/// This function examines the post's reply_to field to see if it references
/// any of the user's post IDs.
///
/// # Arguments
///
/// * `post` - The post to check
/// * `user_posts` - The user's posts to check against
///
/// # Returns
///
/// `true` if the post is a reply to any of the user's posts, `false` otherwise
fn is_reply_to_user(post: &Post, user_posts: &Vec<Rc<RefCell<Post>>>) -> bool {
    if let Some(reply_to) = post.reply_to() {
        // Extract the post ID from the reply_to URL
        let reply_id = if let Some(hash_pos) = reply_to.rfind('#') {
            &reply_to[hash_pos + 1..]
        } else {
            reply_to
        };

        // Check if any user post matches this reply ID
        return user_posts.iter().any(|user_post| user_post.borrow().id() == reply_id);
    }

    false
}

impl std::fmt::Display for NotificationFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Notification Feed with {} notifications:", self.notifications.len())?;
        
        for (i, notification) in self.notifications.iter().enumerate() {
            writeln!(f, "--- Notification {} ({:?}) ---", i + 1, notification.notification_type)?;
            let post_ref = notification.post.borrow();
            if let Some(time) = post_ref.time() {
                writeln!(f, "Time: {time}")?;
            }
            if let Some(author) = post_ref.author() {
                writeln!(f, "Author: {author}")?;
            }
            if let Some(source) = post_ref.source() {
                writeln!(f, "Source: {source}")?;
            }
            writeln!(f, "{}", post_ref)?;
            writeln!(f)?;
        }
        
        Ok(())
    }
}

