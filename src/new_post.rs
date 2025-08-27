//! New post module for creating posts in the org-social TUI.
//!
//! This module contains all the logic for creating, managing, and saving new posts
//! to org-social files.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use crate::util;
use crate::post::Post;

#[derive(Clone, PartialEq)]
pub enum NewPostField {
    Content,
    Tags,
    Mood,
    Lang,
    PollEnd,
    PollOption,
}

pub struct NewPostState {
    pub content: String,
    pub tags: Vec<String>,
    pub tags_input: String,
    pub mood: String,
    pub lang: String,
    pub poll_end: String,
    pub poll_option: String,
    pub current_field: NewPostField,
    // Cursor positions for each field
    pub content_cursor: usize,
    pub tags_input_cursor: usize,
    pub mood_cursor: usize,
    pub lang_cursor: usize,
    pub poll_end_cursor: usize,
    pub poll_option_cursor: usize,
}

impl NewPostState {
    pub fn new(initial_tags: Option<Vec<String>>) -> Self {
        Self {
            content: String::new(),
            tags: initial_tags.unwrap_or_default(),
            tags_input: String::new(),
            mood: String::new(),
            lang: String::new(),
            poll_end: String::new(),
            poll_option: String::new(),
            current_field: NewPostField::Content,
            content_cursor: 0,
            tags_input_cursor: 0,
            mood_cursor: 0,
            lang_cursor: 0,
            poll_end_cursor: 0,
            poll_option_cursor: 0,
        }
    }

    pub fn handle_input(&mut self, c: char) {
        match self.current_field {
            NewPostField::Content => {
                self.content.insert(self.content_cursor, c);
                self.content_cursor += 1;
            }
            NewPostField::Tags => {
                self.tags_input.insert(self.tags_input_cursor, c);
                self.tags_input_cursor += 1;
            }
            NewPostField::Mood => {
                self.mood.insert(self.mood_cursor, c);
                self.mood_cursor += 1;
            }
            NewPostField::Lang => {
                self.lang.insert(self.lang_cursor, c);
                self.lang_cursor += 1;
            }
            NewPostField::PollEnd => {
                self.poll_end.insert(self.poll_end_cursor, c);
                self.poll_end_cursor += 1;
            }
            NewPostField::PollOption => {
                self.poll_option.insert(self.poll_option_cursor, c);
                self.poll_option_cursor += 1;
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.current_field {
            NewPostField::Content => {
                if self.content_cursor > 0 {
                    self.content_cursor -= 1;
                    self.content.remove(self.content_cursor);
                }
            }
            NewPostField::Tags => {
                if self.tags_input_cursor > 0 {
                    self.tags_input_cursor -= 1;
                    self.tags_input.remove(self.tags_input_cursor);
                }
            }
            NewPostField::Mood => {
                if self.mood_cursor > 0 {
                    self.mood_cursor -= 1;
                    self.mood.remove(self.mood_cursor);
                }
            }
            NewPostField::Lang => {
                if self.lang_cursor > 0 {
                    self.lang_cursor -= 1;
                    self.lang.remove(self.lang_cursor);
                }
            }
            NewPostField::PollEnd => {
                if self.poll_end_cursor > 0 {
                    self.poll_end_cursor -= 1;
                    self.poll_end.remove(self.poll_end_cursor);
                }
            }
            NewPostField::PollOption => {
                if self.poll_option_cursor > 0 {
                    self.poll_option_cursor -= 1;
                    self.poll_option.remove(self.poll_option_cursor);
                }
            }
        }
    }

    pub fn handle_newline(&mut self) {
        match self.current_field {
            NewPostField::Content => {
                self.content.insert(self.content_cursor, '\n');
                self.content_cursor += 1;
            }
            _ => {}
        }
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            NewPostField::Content => NewPostField::Tags,
            NewPostField::Tags => NewPostField::Mood,
            NewPostField::Mood => NewPostField::Lang,
            NewPostField::Lang => NewPostField::PollEnd,
            NewPostField::PollEnd => NewPostField::PollOption,
            NewPostField::PollOption => NewPostField::Content,
        };
        self.update_cursor_position();
    }

    pub fn prev_field(&mut self) {
        self.current_field = match self.current_field {
            NewPostField::Content => NewPostField::PollOption,
            NewPostField::Tags => NewPostField::Content,
            NewPostField::Mood => NewPostField::Tags,
            NewPostField::Lang => NewPostField::Mood,
            NewPostField::PollEnd => NewPostField::Lang,
            NewPostField::PollOption => NewPostField::PollEnd,
        };
        self.update_cursor_position();
    }

    fn update_cursor_position(&mut self) {
        match self.current_field {
            NewPostField::Content => {
                self.content_cursor = self.content.len();
            }
            NewPostField::Tags => {
                self.tags_input_cursor = self.tags_input.len();
            }
            NewPostField::Mood => {
                self.mood_cursor = self.mood.len();
            }
            NewPostField::Lang => {
                self.lang_cursor = self.lang.len();
            }
            NewPostField::PollEnd => {
                self.poll_end_cursor = self.poll_end.len();
            }
            NewPostField::PollOption => {
                self.poll_option_cursor = self.poll_option.len();
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
            NewPostField::Content => self.content_cursor,
            NewPostField::Tags => self.tags_input_cursor,
            NewPostField::Mood => self.mood_cursor,
            NewPostField::Lang => self.lang_cursor,
            NewPostField::PollEnd => self.poll_end_cursor,
            NewPostField::PollOption => self.poll_option_cursor,
        }
    }

    pub fn get_current_field_text(&self) -> &str {
        match self.current_field {
            NewPostField::Content => &self.content,
            NewPostField::Tags => &self.tags_input,
            NewPostField::Mood => &self.mood,
            NewPostField::Lang => &self.lang,
            NewPostField::PollEnd => &self.poll_end,
            NewPostField::PollOption => &self.poll_option,
        }
    }

    pub fn is_ready_to_submit(&self) -> bool {
        !self.content.trim().is_empty()
    }

    pub fn create_new_post(&self) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = util::get_current_timestamp();
        
        let mut post = Post::new(timestamp, self.content.clone());
        
        // Set the optional fields if they have values
        if !self.tags.is_empty() {
            post.set_tags(Some(self.tags.clone()));
        }
        
        post.set_client(Some("org-social-rs".to_string()));
        
        if !self.mood.trim().is_empty() {
            post.set_mood(Some(self.mood.trim().to_string()));
        }
        
        if !self.lang.trim().is_empty() {
            post.set_lang(Some(self.lang.trim().to_string()));
        }
        
        if !self.poll_end.trim().is_empty() {
            post.set_poll_end(Some(self.poll_end.trim().to_string()));
        }
        
        if !self.poll_option.trim().is_empty() {
            post.set_poll_option(Some(self.poll_option.trim().to_string()));
        }
        
        Ok(format!("\n{}", post.to_org_social()))
    }
}

pub struct NewPostManager {
    file_path: String,
}

impl NewPostManager {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn save_new_post(&self, new_post_state: &NewPostState) -> Result<String, Box<dyn std::error::Error>> {
        let post_text = new_post_state.create_new_post()?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
            
        writeln!(file, "{post_text}")?;
        
        // Return a preview of the saved content
        let preview = if new_post_state.content.len() > 50 {
            format!("{}...", &new_post_state.content[..50])
        } else {
            new_post_state.content.clone()
        };
        
        Ok(format!("New post saved to {}: {}", self.file_path, preview))
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}
