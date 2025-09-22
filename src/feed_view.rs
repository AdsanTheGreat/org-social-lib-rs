//! Feed view trait and implementations for different feed display and filtering strategies.
//!
//! This module defines the `FeedView` trait that allows different views to be created
//! over a shared collection of posts and profiles. Views can filter, sort, and present
//! the data in different ways while sharing the underlying data.

use crate::feed;

/// A trait for different views over feed data.
///
/// Feed views provide different ways to present and interact with the same
/// underlying collection of posts and profiles. Views share references to
/// the posts without cloning the actual post data.
pub trait FeedView {
    /// Update the view with new posts and profiles.
    /// This is called by the parent Feed when the underlying data changes, and when the view is first added.
    fn update_content(&mut self, feed: &feed::Feed);
    
    /// Get the number of posts in this view.
    fn len(&self) -> usize;
    
    /// Check if this view is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get a display name for this view.
    fn view_name(&self) -> &str;
    
    /// Refresh the view - recompute any derived data
    fn refresh(&mut self);
}