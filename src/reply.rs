//! Reply module for handling post replies in the org-social TUI.
//!
//! This module contains all the logic for creating, managing, and saving replies
//! to org-social posts.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use crate::util;
use crate::post::Post;

#[derive(Clone, PartialEq)]
pub enum ReplyField {
    Content,
    Tags,
    Mood,
}

pub struct ReplyState {
    pub content: String,
    pub tags: Vec<String>,
    pub tags_input: String,
    pub mood: String,
    pub current_field: ReplyField,
    pub reply_to_id: String,
    // Cursor positions for each field
    pub content_cursor: usize,
    pub tags_input_cursor: usize,
    pub mood_cursor: usize,
}

impl ReplyState {
    pub fn new(reply_to_id: String, initial_tags: Option<Vec<String>>) -> Self {
        Self {
            content: String::new(),
            tags: initial_tags.unwrap_or_default(),
            tags_input: String::new(),
            mood: String::new(),
            current_field: ReplyField::Content,
            reply_to_id,
            content_cursor: 0,
            tags_input_cursor: 0,
            mood_cursor: 0,
        }
    }

    pub fn handle_input(&mut self, c: char) {
        match self.current_field {
            ReplyField::Content => {
                self.content.insert(self.content_cursor, c);
                self.content_cursor += 1;
            }
            ReplyField::Tags => {
                self.tags_input.insert(self.tags_input_cursor, c);
                self.tags_input_cursor += 1;
            }
            ReplyField::Mood => {
                self.mood.insert(self.mood_cursor, c);
                self.mood_cursor += 1;
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.current_field {
            ReplyField::Content => {
                if self.content_cursor > 0 {
                    self.content_cursor -= 1;
                    self.content.remove(self.content_cursor);
                }
            }
            ReplyField::Tags => {
                if self.tags_input_cursor > 0 {
                    self.tags_input_cursor -= 1;
                    self.tags_input.remove(self.tags_input_cursor);
                }
            }
            ReplyField::Mood => {
                if self.mood_cursor > 0 {
                    self.mood_cursor -= 1;
                    self.mood.remove(self.mood_cursor);
                }
            }
        }
    }

    pub fn handle_newline(&mut self) {
        match self.current_field {
            ReplyField::Content => {
                self.content.insert(self.content_cursor, '\n');
                self.content_cursor += 1;
            }
            _ => {}
        }
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            ReplyField::Content => ReplyField::Tags,
            ReplyField::Tags => ReplyField::Mood,
            ReplyField::Mood => ReplyField::Content,
        };
        self.update_cursor_position();
    }

    pub fn prev_field(&mut self) {
        self.current_field = match self.current_field {
            ReplyField::Content => ReplyField::Mood,
            ReplyField::Tags => ReplyField::Content,
            ReplyField::Mood => ReplyField::Tags,
        };
        self.update_cursor_position();
    }

    fn update_cursor_position(&mut self) {
        match self.current_field {
            ReplyField::Content => {
                self.content_cursor = self.content.len();
            }
            ReplyField::Tags => {
                self.tags_input_cursor = self.tags_input.len();
            }
            ReplyField::Mood => {
                self.mood_cursor = self.mood.len();
            }
        }
    }

    pub fn finalize_tags_input(&mut self) {
        if !self.tags_input.trim().is_empty() {
            let new_tags: Vec<String> = self.tags_input
                .split_whitespace()
                .map(|s| s.trim_start_matches('#').to_string())
                .collect();
            self.tags.extend(new_tags);
            self.tags_input.clear();
            self.tags_input_cursor = 0;
        }
    }

    pub fn remove_last_tag(&mut self) {
        self.tags.pop();
    }

    pub fn get_cursor_position(&self) -> usize {
        match self.current_field {
            ReplyField::Content => self.content_cursor,
            ReplyField::Tags => self.tags_input_cursor,
            ReplyField::Mood => self.mood_cursor,
        }
    }

    pub fn get_current_field_text(&self) -> &str {
        match self.current_field {
            ReplyField::Content => &self.content,
            ReplyField::Tags => &self.tags_input,
            ReplyField::Mood => &self.mood,
        }
    }

    pub fn is_ready_to_submit(&self) -> bool {
        !self.content.trim().is_empty()
    }

    pub fn create_reply_post(&self) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = util::get_current_timestamp();
        
        let mut post = Post::new(timestamp, self.content.clone());
        
        // Set the reply-specific fields
        if !self.tags.is_empty() {
            post.set_tags(Some(self.tags.clone()));
        }
        
        post.set_client(Some("org-social-rs".to_string()));
        post.set_reply_to(Some(self.reply_to_id.clone()));
        
        if !self.mood.trim().is_empty() {
            post.set_mood(Some(self.mood.trim().to_string()));
        }
        
        Ok(format!("\n{}", post.to_org_social()))
    }
}

pub struct ReplyManager {
    file_path: String,
}

impl ReplyManager {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn save_reply(&self, reply_state: &ReplyState) -> Result<String, Box<dyn std::error::Error>> {
        let reply_text = reply_state.create_reply_post()?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
            
        writeln!(file, "{reply_text}")?;
        
        // Return a preview of the saved content
        let preview = if reply_state.content.len() > 50 {
            format!("{}...", &reply_state.content[..50])
        } else {
            reply_state.content.clone()
        };
        
        Ok(format!("Reply saved to {}: {}", self.file_path, preview))
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}
