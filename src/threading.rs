//! Threading module for creating tree views of org-social posts.
//!
//! This module provides functionality to organize posts into threaded conversations
//! based on reply relationships, creating hierarchical tree structures for display.

use crate::{feed::Feed, poll::Poll, post::Post};
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;

/// Represents a node in a threaded conversation tree.
#[derive(Clone)]
pub struct ThreadNode {
    /// The post at this node
    pub post: Post,
    /// Direct replies to this post
    pub replies: Vec<ThreadNode>,
    /// Depth level in the conversation (0 = root)
    pub depth: usize,
    /// Latest activity time in this node's subtree (including this post and all replies)
    pub latest_activity_time: Option<DateTime<FixedOffset>>,
}

/// Represents a collection of threaded conversations.
pub struct ThreadView<'a> {
    /// Reference to the base feed
    pub feed: &'a Feed,
    /// Root posts (posts that are not replies to anything)
    pub roots: Vec<ThreadNode>,
    /// Map of post IDs to their full identifiers for quick lookup
    id_map: HashMap<String, String>,
    /// Temporary map for placeholder posts during construction
    placeholder_map: HashMap<String, ThreadNode>,
}

impl ThreadNode {
    pub fn new(post: Post, depth: usize) -> Self {
        let latest_activity_time = post.time();
        Self {
            post,
            replies: Vec::new(),
            depth,
            latest_activity_time,
        }
    }

    pub fn add_reply(&mut self, reply_node: ThreadNode) {
        self.replies.push(reply_node);
    }

    /// Calculate and update the latest activity time for this node and all its descendants.
    /// This should be called after the tree structure is complete.
    pub fn update_latest_activity_time(&mut self) {
        // Everybody loves recursion, right?
        for reply in &mut self.replies {
            reply.update_latest_activity_time();
        }
        
        // Start with this post's own time
        let mut latest_time = self.post.time();
        
        // Check all replies for later times
        for reply in &self.replies {
            match (latest_time, reply.latest_activity_time) {
                (Some(current), Some(reply_time)) => {
                    if reply_time > current {
                        latest_time = Some(reply_time);
                    }
                }
                (None, Some(reply_time)) => {
                    latest_time = Some(reply_time);
                }
                _ => {} // Keep current latest_time
            }
        }
        
        self.latest_activity_time = latest_time;
    }

    pub fn sort_replies(&mut self) {
        self.replies.sort_by(|a, b| {
            match (a.latest_activity_time, b.latest_activity_time) {
                (Some(time_a), Some(time_b)) => time_a.cmp(&time_b), // Chronological order
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        // Recursively sort replies of replies
        for reply in &mut self.replies {
            reply.sort_replies();
        }
    }

    pub fn count_posts(&self) -> usize {
        1 + self.replies.iter().map(|r| r.count_posts()).sum::<usize>()
    }

    pub fn flatten(&self) -> Vec<&Post> {
        let mut posts = vec![&self.post];
        for reply in &self.replies {
            posts.extend(reply.flatten());
        }
        posts
    }
}

impl<'a> ThreadView<'a> {
    pub fn new(feed: &'a Feed) -> Self {
        Self {
            feed,
            roots: Vec::new(),
            id_map: HashMap::new(),
            placeholder_map: HashMap::new(),
        }
    }

    /// Create a threaded view from a Feed.
    pub fn from_feed(feed: &'a Feed) -> Self {
        let mut thread_view = Self::new(feed);
        let mut post_map: HashMap<String, ThreadNode> = HashMap::new();
        let mut reply_map: HashMap<String, Vec<ThreadNode>> = HashMap::new();

        // Build ID mapping for quick lookups
        for post in &feed.posts {
            let full_id: String = post.full_id();
            thread_view.id_map.insert(post.id().to_string(), full_id);
        }

        // First pass: create nodes for all posts
        for post in &feed.posts {
            let node = ThreadNode::new(post.clone(), 0);
            let full_id = post.full_id();
            post_map.insert(full_id, node);
        }

        // Second pass: organize into threads and create placeholders for missing parents
        let post_map_clone = post_map.clone();
        for (_post_id, mut node) in post_map {
            if let Some(reply_to) = node.post.reply_to() {
                // This is a reply to another post
                let reply_target = Self::resolve_reply_target(reply_to, &thread_view.id_map);
                
                if let Some(parent_node) = post_map_clone.get(&reply_target) {
                    // Parent exists, add to reply map
                    node.depth = parent_node.depth + 1;
                    reply_map.entry(reply_target).or_default().push(node);
                } else {
                    // Parent doesn't exist, try timestamp-based fallback
                    if let Some(fallback_target) = Self::find_by_timestamp_fallback(&reply_target, &post_map_clone) {
                        // Found a post with matching timestamp, use it
                        node.depth = post_map_clone.get(&fallback_target).unwrap().depth + 1;
                        reply_map.entry(fallback_target).or_default().push(node);
                    } else {
                        // No match found even by timestamp, create a placeholder
                        let placeholder_post = Self::create_placeholder_post(&reply_target);
                        let placeholder_node = ThreadNode::new(placeholder_post.clone(), 0);
                        node.depth = 1; // Reply to placeholder at depth 0
                        
                        // Add placeholder to placeholder_map and this node as its reply
                        thread_view.placeholder_map.insert(reply_target.clone(), placeholder_node);
                        reply_map.entry(reply_target).or_default().push(node);
                    }
                }
            } else {
                // This is a root post
                thread_view.roots.push(node);
            }
        }

        // Third pass: attach replies to their parents (including placeholders)
        thread_view.attach_replies(&reply_map);

        // Fourth pass: attach replies to placeholder nodes and move them to roots
        for (_, mut placeholder_node) in thread_view.placeholder_map.clone() {
            Self::attach_replies_to_node(&mut placeholder_node, &reply_map);
            thread_view.roots.push(placeholder_node);
        }

        // Sort all threads
        thread_view.sort_threads();

        thread_view
    }

    /// Resolve a reply_to target to a full post identifier.
    fn resolve_reply_target(reply_to: &str, id_map: &HashMap<String, String>) -> String {
        if reply_to.contains('#') {
            // Already a full identifier (url#id or nick#id)
            reply_to.to_string()
        } else {
            // Just an ID, look it up in the map
            id_map.get(reply_to).cloned().unwrap_or_else(|| reply_to.to_string())
        }
    }

    /// Attempt to find a post by timestamp-only matching when the full ID is not found.
    /// 
    /// This extracts the timestamp portion from the reply target and searches for any
    /// post with a matching timestamp ID, regardless of source.
    ///
    /// # Arguments
    /// * `reply_target` - The full reply target that couldn't be found
    /// * `post_map` - Map of all available posts
    ///
    /// # Returns
    /// The full ID of a matching post if found, None otherwise
    fn find_by_timestamp_fallback(reply_target: &str, post_map: &HashMap<String, ThreadNode>) -> Option<String> {
        // Extract timestamp from reply target
        let timestamp = if reply_target.contains('#') {
            // Extract the part after the last '#' which should be the timestamp
            reply_target.split('#').next_back()?
        } else {
            // Already just a timestamp
            reply_target
        };

        // Search through all posts for one with this timestamp as the ID
        for (full_id, node) in post_map {
            if node.post.id() == timestamp {
                return Some(full_id.clone());
            }
        }

        None
    }

    /// Create a placeholder post for missing reply targets.
    fn create_placeholder_post(reply_target: &str) -> Post {
        let placeholder_id = if reply_target.contains('#') {
            // Extract just the ID part after the hash
            reply_target.split('#').next_back().unwrap_or("unknown").to_string()
        } else {
            reply_target.to_string()
        };
        
        let mut placeholder = Post::new(placeholder_id, "[Post not available]".to_string());
        placeholder.set_author("unknown".to_string());
        
        // If the reply_target has a source part (before #), set it
        if let Some(hash_pos) = reply_target.find('#') {
            let source = &reply_target[..hash_pos];
            if !source.is_empty() {
                placeholder.set_source(Some(source.to_string()));
            }
        }
        
        placeholder
    }

    /// Attach replies to their parent nodes recursively.
    fn attach_replies(&mut self, reply_map: &HashMap<String, Vec<ThreadNode>>) {
        // Attach replies to root nodes
        for root in &mut self.roots {
            Self::attach_replies_to_node(root, reply_map);
        }
    }

    /// Recursively attach replies to a specific node.
    fn attach_replies_to_node(node: &mut ThreadNode, reply_map: &HashMap<String, Vec<ThreadNode>>) {
        let node_id = node.post.full_id();
        if let Some(replies) = reply_map.get(&node_id) {
            for mut reply in replies.clone() {
                Self::attach_replies_to_node(&mut reply, reply_map);
                node.add_reply(reply);
            }
        }
    }

    /// Sort all threads and their replies chronologically.
    pub fn sort_threads(&mut self) {
        // First, update latest activity times for all threads
        for root in &mut self.roots {
            root.update_latest_activity_time();
        }
        
        // Sort root posts (latest activity first)
        self.roots.sort_by(|a, b| {
            match (a.latest_activity_time, b.latest_activity_time) {
                (Some(time_a), Some(time_b)) => time_b.cmp(&time_a), // Reverse for latest activity first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        // Sort replies within each thread
        for root in &mut self.roots {
            root.sort_replies();
        }
    }

    pub fn thread_count(&self) -> usize {
        self.roots.len()
    }

    pub fn total_posts(&self) -> usize {
        self.roots.iter().map(|r| r.count_posts()).sum()
    }

    pub fn flatten(&self) -> Vec<&Post> {
        let mut posts = Vec::new();
        for root in &self.roots {
            posts.extend(root.flatten());
        }
        posts
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn update_poll_node(&self, post_node: &ThreadNode, poll: &mut Poll) {
        poll.clear_votes();
        for reply in &post_node.replies {
            poll.add_vote_from_reply(&reply.post);
        }
    }

    /// Add a new post to the thread tree.
    /// 
    /// If the post is a reply, it will be added to the appropriate parent node.
    /// If the parent doesn't exist, a placeholder will be created.
    /// If it's not a reply, it will be added as a new root thread.
    /// 
    /// After adding the post, latest activity times will be updated and threads will be re-sorted.
    ///
    /// # Arguments
    /// * `post` - The new post to add to the thread tree
    pub fn add_post(&mut self, post: Post) {
        if let Some(reply_to) = post.reply_to() {
            let reply_target = Self::resolve_reply_target(reply_to, &self.id_map);
            
            // Try to find the parent in existing threads
            if self.find_and_add_reply(&reply_target, post.clone()).is_some() {
                self.id_map.insert(post.id().to_string(), post.full_id());
                
                self.sort_threads();
            } else {
                // Parent not found - create placeholder and add as new root thread
                let placeholder_post = Self::create_placeholder_post(&reply_target);
                let mut placeholder_node = ThreadNode::new(placeholder_post, 0);
                
                let reply_node = ThreadNode::new(post.clone(), 1);
                placeholder_node.add_reply(reply_node);
                
                placeholder_node.update_latest_activity_time();
                
                self.roots.push(placeholder_node);
                
                self.id_map.insert(post.id().to_string(), post.full_id());
                
                // Resort threads
                self.sort_threads();
            }
        } else {
            // This is a root post
            let new_root = ThreadNode::new(post.clone(), 0);
            self.roots.push(new_root);
            
            self.id_map.insert(post.id().to_string(), post.full_id());
            
            // Resort threads
            self.sort_threads();
        }
    }

    /// Recursively search for a target post ID and add a reply to it.
    /// Returns Some(depth) if the reply was successfully added, None if target not found.
    fn find_and_add_reply(&mut self, target_id: &str, reply_post: Post) -> Option<usize> {
        for root in &mut self.roots {
            if let Some(depth) = Self::find_and_add_reply_to_node(root, target_id, reply_post.clone()) {
                return Some(depth);
            }
        }
        None
    }

    /// Recursively search within a specific node and its descendants for the target ID.
    /// Returns Some(depth) if the reply was successfully added, None if target not found.
    fn find_and_add_reply_to_node(node: &mut ThreadNode, target_id: &str, reply_post: Post) -> Option<usize> {
        // Check if this node is the target
        if node.post.full_id() == target_id {
            let reply_depth = node.depth + 1;
            let reply_node = ThreadNode::new(reply_post, reply_depth);
            node.add_reply(reply_node);
            
            // Update latest activity time for this node and propagate upwards
            node.update_latest_activity_time();
            
            return Some(reply_depth);
        }
        
        // Search in replies
        for reply in &mut node.replies {
            if let Some(depth) = Self::find_and_add_reply_to_node(reply, target_id, reply_post.clone()) {
                // Update our latest activity time
                node.update_latest_activity_time();
                return Some(depth);
            }
        }
        
        None
    }
}

impl<'a> From<&'a Feed> for ThreadView<'a> {
    fn from(feed: &'a Feed) -> Self {
        ThreadView::from_feed(feed)
    }
}

impl<'a> Default for ThreadView<'a> {
    fn default() -> Self {
        panic!("ThreadView::default() is not supported, use ThreadView::new(feed)");
    }
}

impl<'a> std::fmt::Display for ThreadView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Thread View with {} conversations:", self.thread_count())?;
        
        for (i, root) in self.roots.iter().enumerate() {
            writeln!(f, "\n--- Thread {} ---", i + 1)?;
            Self::display_node(f, root, "")?;
        }
        
        Ok(())
    }
}

impl<'a> ThreadView<'a> {
    /// Helper method to display a thread node with proper indentation.
    fn display_node(f: &mut std::fmt::Formatter<'_>, node: &ThreadNode, prefix: &str) -> std::fmt::Result {
        // Display the post with indentation
        let indent = "  ".repeat(node.depth);
        writeln!(f, "{}{}Post ID: {}", prefix, indent, node.post.id())?;
        
        if let Some(time) = node.post.time() {
            writeln!(f, "{prefix}{indent}Time: {time}")?;
        }
        
        if let Some(author) = node.post.author() {
            writeln!(f, "{prefix}{indent}Author: {author}")?;
        }
        
        writeln!(f, "{}{}Content: {}", prefix, indent, node.post.content())?;
        
        // Display replies
        for reply in &node.replies {
            Self::display_node(f, reply, prefix)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post::Post;

    #[test]
    fn test_placeholder_parent_linking() {
        // Create a post that replies to a non-existent post
        let mut reply_post = Post::new("reply1".to_string(), "This is a reply".to_string());
        reply_post.set_reply_to(Some("missing_post".to_string()));
        reply_post.set_author("user1".to_string());

        let posts = vec![reply_post.clone()];
        
        // Create thread view
        let feed = crate::feed::Feed { posts: posts.clone(), profiles: vec![], profile_map: std::collections::HashMap::new() };
        let thread_view = ThreadView::from_feed(&feed);
        
        // Should have one root thread (the placeholder)
        assert_eq!(thread_view.thread_count(), 1);
        
        // The root should be a placeholder post
        let root = &thread_view.roots[0];
        assert_eq!(root.post.id(), "missing_post");
        assert_eq!(root.post.content(), "[Post not available]");
        assert_eq!(root.post.author().as_deref(), Some("unknown"));
        
        // The placeholder should have one reply
        assert_eq!(root.replies.len(), 1);
        
        // The reply should be our original post
        let reply_node = &root.replies[0];
        assert_eq!(reply_node.post.id(), "reply1");
        assert_eq!(reply_node.post.content(), "This is a reply");
        assert_eq!(reply_node.depth, 1);
    }

    #[test]
    fn test_multiple_replies_to_missing_post() {
        // Create multiple posts that reply to the same non-existent post
        let mut reply1 = Post::new("reply1".to_string(), "First reply".to_string());
        reply1.set_reply_to(Some("missing_post".to_string()));
        
        let mut reply2 = Post::new("reply2".to_string(), "Second reply".to_string());
        reply2.set_reply_to(Some("missing_post".to_string()));

        let posts = vec![reply1, reply2];
        
        // Create thread view
        let feed = crate::feed::Feed { posts: posts.clone(), profiles: vec![], profile_map: std::collections::HashMap::new() };
        let thread_view = ThreadView::from_feed(&feed);
        
        // Should have one root thread (the placeholder)
        assert_eq!(thread_view.thread_count(), 1);
        
        // The root should be a placeholder post with two replies
        let root = &thread_view.roots[0];
        assert_eq!(root.post.id(), "missing_post");
        assert_eq!(root.replies.len(), 2);
        
        // Both replies should be at depth 1
        for reply in &root.replies {
            assert_eq!(reply.depth, 1);
        }
    }

    #[test]
    fn test_timestamp_fallback_matching() {
        // Create a post with a timestamp ID
        let original_post = Post::new("2025-08-15T10:30:00+00:00".to_string(), "Original post".to_string());
        
        // Create a reply that targets the same timestamp but from a different source
        let mut reply_post = Post::new("reply1".to_string(), "This is a reply".to_string());
        reply_post.set_reply_to(Some("https://external.site/social.org/#2025-08-15T10:30:00+00:00".to_string()));
        reply_post.set_author("user1".to_string());

        let posts = vec![original_post.clone(), reply_post.clone()];
        
        // Create thread view
        let feed = crate::feed::Feed { posts: posts.clone(), profiles: vec![], profile_map: std::collections::HashMap::new() };
        let thread_view = ThreadView::from_feed(&feed);
        
        // Should have one root thread (the original post)
        assert_eq!(thread_view.thread_count(), 1);
        
        // The root should be the original post
        let root = &thread_view.roots[0];
        assert_eq!(root.post.id(), "2025-08-15T10:30:00+00:00");
        assert_eq!(root.post.content(), "Original post");
        
        // The original post should have one reply
        assert_eq!(root.replies.len(), 1);
        
        // The reply should be our reply post
        let reply_node = &root.replies[0];
        assert_eq!(reply_node.post.id(), "reply1");
        assert_eq!(reply_node.post.content(), "This is a reply");
        assert_eq!(reply_node.depth, 1);
    }

    #[test]
    fn test_timestamp_fallback_no_match_creates_placeholder() {
        // Create a reply that targets a timestamp that doesn't exist
        let mut reply_post = Post::new("reply1".to_string(), "This is a reply".to_string());
        reply_post.set_reply_to(Some("https://external.site/social.org/#2025-12-25T00:00:00+00:00".to_string()));
        reply_post.set_author("user1".to_string());

        let posts = vec![reply_post.clone()];
        
        // Create thread view
        let feed = crate::feed::Feed { posts: posts.clone(), profiles: vec![], profile_map: std::collections::HashMap::new() };
        let thread_view = ThreadView::from_feed(&feed);
        
        // Should have one root thread (the placeholder)
        assert_eq!(thread_view.thread_count(), 1);
        
        // The root should be a placeholder post
        let root = &thread_view.roots[0];
        assert_eq!(root.post.id(), "2025-12-25T00:00:00+00:00");
        assert_eq!(root.post.content(), "[Post not available]");
        assert_eq!(root.post.author().as_deref(), Some("unknown"));
        
        // The placeholder should have one reply
        assert_eq!(root.replies.len(), 1);
        
        // The reply should be our original post
        let reply_node = &root.replies[0];
        assert_eq!(reply_node.post.id(), "reply1");
        assert_eq!(reply_node.post.content(), "This is a reply");
        assert_eq!(reply_node.depth, 1);
    }
}