//! New post module for creating posts in the org-social TUI.
//!
//! This module is a helpful wrapper around the Post struct to facilitate
//! easier creation of post editors.
//! It aggregates the functionality needed to create posts, replies, votes, etc.

use crate::util;
use crate::post::Post;

pub struct NewPostState {
    pub lang: String,
    pub tags: Vec<String>,
    pub mood: String,

    pub content: String,

    pub reply_to: Option<String>,
    pub poll_end: Option<String>,
    pub poll_option: Option<String>,
}

impl NewPostState {
    pub fn new() -> Self {
        Self {
            lang: String::new(),
            tags: Vec::new(),
            mood: String::new(),
            content: String::new(),
            reply_to: None,
            poll_end: None,
            poll_option: None,
        }
    }

    pub fn reply(reply_to: String, initial_tags: Option<Vec<String>>) -> Self {
        Self {
            lang: String::new(),
            tags: initial_tags.unwrap_or_default(),
            mood: String::new(),
            content: String::new(),
            reply_to: Some(reply_to),
            poll_end: None,
            poll_option: None,
        }
    }

    pub fn reply_to_post(target_post: Post) -> Self {
        Self::reply(
            target_post.full_id(),
            target_post.tags().clone()
        )
    }

    pub fn vote(reply_to: String, initial_tags: Option<Vec<String>>, poll_option: String) -> Self {
        Self {
            lang: String::new(),
            tags: initial_tags.unwrap_or_default(),
            mood: String::new(),
            content: String::new(),
            reply_to: Some(reply_to),
            poll_end: None,
            poll_option: Some(poll_option),
        }
    }

    pub fn vote_on_post(target_post: Post, poll_option: String) -> Self {
        Self::vote(
            target_post.full_id(), 
            target_post.tags().clone(), 
            poll_option
        )
    }

    pub fn is_empty(&self) -> bool {
        !self.content.trim().is_empty()
    }

    pub fn is_reply(&self) -> bool {
        self.reply_to.is_some()
    }

    pub fn is_vote(&self) -> bool {
        self.poll_option.is_some()
    }

    /// TODO: Implement poll creation
    pub fn is_poll(&self) -> bool {
        self.poll_end.is_some()
    }

    pub fn create_post(&self, client_name: &str) -> Post {
        let timestamp = util::get_current_timestamp();
        

        let mut post = Post::new(timestamp, self.content.clone());
        post.set_lang(if self.lang.is_empty() { None } else { Some(self.lang.clone()) });
        post.set_tags(if self.tags.is_empty() { None } else { Some(self.tags.clone()) });
        post.set_mood(if self.mood.is_empty() { None } else { Some(self.mood.clone()) });
        post.set_reply_to(self.reply_to.clone());
        post.set_poll_end(self.poll_end.clone());
        post.set_poll_option(self.poll_option.clone());
        post.set_client(Some(client_name.to_string()));

        post
    }
}
