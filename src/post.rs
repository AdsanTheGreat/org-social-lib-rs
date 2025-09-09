//! Post module for org-social file format.
//! 
//! This module contains the Post struct and its implementations
//! for parsing and serializing org-social posts.

use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::Write;

use chrono::{DateTime, FixedOffset};

use crate::profile::Profile;
use crate::util;
use crate::tokenizer::{Token, Tokenizer};
use crate::blocks::{ActivatableElement, parse_blocks_with_poll_end};

/// Represents a post parsed from an org-social file.
/// 
/// Contains post metadata, it's content, author and source information,
/// as well as parsed tokens and blocks from the content.
#[derive(Clone, Debug)]
#[derive(Default)]
pub struct Post {
    id: String,
    lang: Option<String>,
    tags: Option<Vec<String>>,
    client: Option<String>,
    reply_to: Option<String>,
    poll_end: Option<String>,
    poll_option: Option<String>,
    mood: Option<String>,
    pub(crate) content: String,
    source: Option<String>,
    author: Option<String>,
    tokens: Vec<Token>,
    blocks: Vec<ActivatableElement>,
}


impl From<&Post> for Post {
    fn from(post: &Post) -> Self {
        Post {
            id: post.id.clone(),
            lang: post.lang.clone(),
            tags: post.tags.clone(),
            client: post.client.clone(),
            reply_to: post.reply_to.clone(),
            poll_end: post.poll_end.clone(),
            poll_option: post.poll_option.clone(),
            mood: post.mood.clone(),
            content: post.content.clone(),
            source: post.source.clone(),
            author: post.author.clone(),
            tokens: post.tokens.clone(),
            blocks: post.blocks.clone(),
        }
    }
}

impl From<Vec<String>> for Post {
    /// Parse a post from the org-social format.
    ///
    /// Extracts post metadata from property blocks and content from the body.
    fn from(post_section_lines: Vec<String>) -> Self {
        let mut id = String::new();
        let mut lang: Option<String> = None;
        let mut tags: Option<Vec<String>> = None;
        let mut client: Option<String> = None;
        let mut reply_to: Option<String> = None;
        let mut poll_end: Option<String> = None;
        let mut poll_option: Option<String> = None;
        let mut mood: Option<String> = None;
        let mut content = String::new();

        let mut in_properties = false;
        let mut properties_ended = false;

        for line in &post_section_lines {
            // Thanks to @omidmash and his own interpretation of the specification, PROPERTIES can also be in the same line as **
            if line.starts_with("** :PROPERTIES:") || line.starts_with(":PROPERTIES:") {
                in_properties = true;
                continue;
            }
            
            if line.starts_with(":END:") {
                if in_properties {
                    properties_ended = true;
                    in_properties = false;
                }
                continue;
            }

            if line.trim() == "**" {
                continue;
            }
            
            if in_properties && line.starts_with(':') {
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    match parts[0].trim() {
                        ":ID" => id = parts[1].trim().to_string(),
                        ":LANG" => lang = Some(parts[1].trim().to_string()),
                        ":TAGS" => {
                            if tags.is_none() {
                                tags = Some(Vec::new());
                            }
                            tags.as_mut()
                                .unwrap()
                                .extend(parts[1].split_whitespace().map(String::from));
                        }
                        ":CLIENT" => client = Some(parts[1].trim().to_string()),
                        ":REPLY_TO" => reply_to = Some(parts[1].trim().to_string()),
                        ":POLL_END" => poll_end = Some(parts[1].trim().to_string()),
                        ":POLL_OPTION" => poll_option = Some(parts[1].trim().to_string()),
                        ":MOOD" => mood = Some(parts[1].trim().to_string()),
                        _ => {}
                    }
                }
                continue;
            }
            
            // Collect content
            if properties_ended
                && (!content.is_empty() || !line.is_empty()) {
                    content.push_str(line);
                    content.push('\n');
                }
        }

        // Remove trailing newline from content
        if content.ends_with('\n') {
            content.pop();
        }

        let mut post = Post {
            id,
            lang,
            tags,
            client,
            reply_to,
            poll_end,
            poll_option,
            mood,
            content,
            source: None,
            author: None,
            tokens: Vec::new(),
            blocks: Vec::new(),
        };

        post.parse_content();
        
        post
    }
}

impl Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Post:\nID: {}\nLang: {:?}\nTags: {:?}\nClient: {:?}\nReply To: {:?}\nPoll End: {:?}\nPoll Option: {:?}\nMood: {:?}\nSource: {:?}\nAuthor: {:?}\nTokens: {} parsed\nBlocks: {} parsed\nContent:\n{}",
            self.id, self.lang, self.tags, self.client, self.reply_to, self.poll_end, self.poll_option, self.mood, self.source, self.author, self.tokens.len(), self.blocks.len(), self.content
        )
    }
}

impl Post {
    /// Create a new post with the given ID and content.
    /// If the `autotokenize` feature is enabled, the content will be automatically parsed.
    /// Otherwise, tokens and blocks will be empty until manual parsing is invoked.
    pub fn new(id: String, content: String) -> Self {
        #[cfg(feature = "autotokenize")] {
            let mut post = Post {
                id,
                content,
                tokens: Vec::new(),
                blocks: Vec::new(),
                ..Default::default()
            };
        
            post.parse_content();
            return post;
        }
        #[cfg(not(feature = "autotokenize"))] {
            Post {
                id,
                content,
                tokens: Vec::new(),
                blocks: Vec::new(),
                ..Default::default()
            }
        }
    }

    /// Parse the content to extract tokens and blocks.
    pub fn parse_content(&mut self) {
        let mut tokenizer = Tokenizer::new(self.content.clone());
        self.tokens = tokenizer.tokenize();
        
        self.blocks = parse_blocks_with_poll_end(&self.content, self.poll_end.clone());
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn blocks(&self) -> &[ActivatableElement] {
        &self.blocks
    }

    pub fn time(&self) -> Option<DateTime<FixedOffset>> {
        if !self.id.is_empty() {
            util::parse_timestamp(&self.id).ok()
        } else {
            None
        }
    }

    pub fn source(&self) -> &Option<String> {
        &self.source
    }

    pub fn lang(&self) -> &Option<String> {
        &self.lang
    }

    pub fn tags(&self) -> &Option<Vec<String>> {
        &self.tags
    }

    pub fn client(&self) -> &Option<String> {
        &self.client
    }

    pub fn reply_to(&self) -> &Option<String> {
        &self.reply_to
    }

    pub fn poll_end(&self) -> &Option<String> {
        &self.poll_end
    }

    pub fn poll_option(&self) -> &Option<String> {
        &self.poll_option
    }

    pub fn mood(&self) -> &Option<String> {
        &self.mood
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn author(&self) -> &Option<String> {
        &self.author
    }

    pub fn set_author(&mut self, author: String) {
        self.author = Some(author);
    }

    pub fn set_source(&mut self, source: Option<String>) {
        self.source = source;
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    // Automatically re-parses the new content.
    /// If the `autotokenize` feature is enabled, the content will be automatically parsed.
    /// Otherwise, tokens and blocks will be cleared until manual parsing is invoked.
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        #[cfg(feature = "autotokenize")]
        {
            self.parse_content();
        }
        #[cfg(not(feature = "autotokenize"))]
        {
            self.tokens.clear();
            self.blocks.clear();
        }
    }

    pub fn set_tags(&mut self, tags: Option<Vec<String>>) {
        self.tags = tags;
    }

    pub fn set_client(&mut self, client: Option<String>) {
        self.client = client;
    }

    pub fn set_reply_to(&mut self, reply_to: Option<String>) {
        self.reply_to = reply_to;
    }

    pub fn set_mood(&mut self, mood: Option<String>) {
        self.mood = mood;
    }

    pub fn set_lang(&mut self, lang: Option<String>) {
        self.lang = lang;
    }

    pub fn set_poll_end(&mut self, poll_end: Option<String>) {
        self.poll_end = poll_end;
    }

    pub fn set_poll_option(&mut self, poll_option: Option<String>) {
        self.poll_option = poll_option;
    }

    pub fn is_poll(&self) -> bool {
        crate::poll::is_poll_post(self)
    }

    pub fn get_poll(&self) -> Option<crate::poll::Poll> {
        crate::poll::parse_poll_from_post(self)
    }

    pub fn is_poll_vote(&self) -> bool {
        self.poll_option.is_some() && self.reply_to.is_some()
    }

    pub fn full_id(&self) -> String {
        if let Some(source) = &self.source {
            format!("{}#{}", source, self.id)
        } else {
            self.id.clone()
        }
    }

    pub fn summary(&self, len: usize) -> String {
        let mut summary = self.content.clone();
        if summary.len() > len {
            summary.truncate(len);
            summary.push_str("...");
        }
        summary
    }

    pub fn format_for_display(&self, profile: Option<&Profile>) -> String {
        let mut output = String::new();

        // Build header line with username, tags, and time
        let mut header = if let Some(author) = &self.author {
            author.to_string()
        } else {
            "unknown".to_string()
        };

        // Add language as first tag if present
        if let Some(lang) = self.lang() {
            header.push_str(&format!(" #{}", lang));
        }

        // Add other tags
        if let Some(tags) = self.tags() {
            for tag in tags {
                header.push_str(&format!(" #{}", tag));
            }
        }

        // Add timestamp if available
        if let Some(time) = self.time() {
            header.push_str(&format!(" • {}", time.format("%Y-%m-%d %H:%M")));
        }

        output.push_str(&format!("--- {} ---\n", header));

        // Collect additional metadata for display
        let mut metadata = Vec::new();

        if let Some(client) = self.client() {
            metadata.push(format!("Client: {}", client));
        }

        if let Some(reply_to) = self.reply_to() {
            // Extract the post ID from the reply_to URL 
            let reply_id = if let Some(hash_pos) = reply_to.rfind('#') {
                &reply_to[hash_pos + 1..]
            } else {
                reply_to
            };
            
            // Try to map the URL to a nickname from the profile's follow list
            let reply_display = if let Some(profile) = profile {
                if let Some(follows) = profile.follow() {
                    // Extract the base URL (without the fragment)
                    let base_url = if let Some(hash_pos) = reply_to.rfind('#') {
                        &reply_to[..hash_pos]
                    } else {
                        reply_to
                    };
                    
                    // Normalize URLs by removing trailing slashes for comparison - they might be included by mistake
                    let normalized_base = base_url.trim_end_matches('/');
                    
                    // Find the nickname for this URL
                    if let Some((nick, _)) = follows.iter().find(|(_, url)| url.trim_end_matches('/') == normalized_base) {
                        format!("{nick}#{reply_id}")
                    } else {
                        // No nickname found, use url#ID format
                        format!("{base_url}#{reply_id}")
                    }
                } else {
                    // No follow list, use url#ID format
                    format!("{}#{}", 
                        if let Some(hash_pos) = reply_to.rfind('#') {
                            &reply_to[..hash_pos]
                        } else {
                            reply_to
                        }, 
                        reply_id)
                }
            } else {
                // No profile, use url#ID format
                format!("{}#{}", 
                    if let Some(hash_pos) = reply_to.rfind('#') {
                        &reply_to[..hash_pos]
                    } else {
                        reply_to
                    }, 
                    reply_id)
            };
            
            metadata.push(format!("Reply to: {}", reply_display));
        }

        if let Some(mood) = self.mood() {
            metadata.push(format!("Mood: {}", mood));
        }

        if let Some(poll_end) = self.poll_end() {
            metadata.push(format!("Poll ends: {}", poll_end));
        }

        if let Some(poll_option) = self.poll_option() {
            metadata.push(format!("Poll option: {}", poll_option));
        }

        // Display metadata if any exists
        if !metadata.is_empty() {
            output.push_str(&format!("{}\n", metadata.join(" | ")));
        }

        // Add post content
        output.push_str(self.content());

        output
    }

    /// Serialize the post to org-social format.
    /// 
    /// Returns the post as org-mode formatted lines including the post header,
    /// properties block, and content.
    pub fn to_org_social(&self) -> String {
        let mut lines = Vec::new();

        lines.push("**".to_string());
        
        lines.push(":PROPERTIES:".to_string());
        
        if !self.id.is_empty() {
            lines.push(format!(":ID: {}", self.id));
        }

        if let Some(lang) = &self.lang {
            lines.push(format!(":LANG: {lang}"));
        }

        if let Some(tags) = &self.tags {
            if !tags.is_empty() {
                lines.push(format!(":TAGS: {}", tags.join(" ")));
            }
        }

        if let Some(client) = &self.client {
            lines.push(format!(":CLIENT: {client}"));
        }

        if let Some(reply_to) = &self.reply_to {
            lines.push(format!(":REPLY_TO: {reply_to}"));
        }

        if let Some(poll_end) = &self.poll_end {
            lines.push(format!(":POLL_END: {poll_end}"));
        }

        if let Some(poll_option) = &self.poll_option {
            lines.push(format!(":POLL_OPTION: {poll_option}"));
        }

        if let Some(mood) = &self.mood {
            lines.push(format!(":MOOD: {mood}"));
        }

        lines.push(":END:".to_string());

        // Empty line before content - for better readability in text mode.
        lines.push("".to_string());

        lines.push(self.content.clone());

        lines.join("\n")
    }

    /// Save the post to the specified file in org-social format.
    pub fn save_post(&self, target_file: &str) -> Result<String, Box<dyn std::error::Error>> {
        let post_text = self.to_org_social();

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(target_file)?;
            
        writeln!(file, "{post_text}")?;

        Ok(format!("New post saved to {}: {}", target_file, self.summary(50)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "autotokenize")]
    #[test]
    fn test_post_self_parsing() {
        let content = "This is *bold* text with /italic/ formatting.\nAnd a second line with ~code~.".to_string();
        let post = Post::new("test-id".to_string(), content.clone());
        
        assert_eq!(post.content(), &content);
        assert!(!post.tokens().is_empty());
        
        let tokens = post.tokens();
        assert!(tokens.iter().any(|t| matches!(t, Token::Bold(_))));
        assert!(tokens.iter().any(|t| matches!(t, Token::Italic(_))));
        assert!(tokens.iter().any(|t| matches!(t, Token::InlineCode(_))));
    }

    #[cfg(feature = "autotokenize")]
    #[test]
    fn test_content_reparsing() {
        let mut post = Post::new("test-id".to_string(), "Initial content".to_string());
        let initial_token_count = post.tokens().len();
        
        // Change content to something with more formatting
        post.set_content("New *bold* content with /italic/ and ~code~".to_string());
        
        // Should have more tokens now
        assert!(post.tokens().len() >= initial_token_count);
        assert!(post.tokens().iter().any(|t| matches!(t, Token::Bold(_))));
    }

    #[test]
    fn test_from_org_social_format_with_multiline() {
        let post_lines = vec![
            "**".to_string(),
            ":PROPERTIES:".to_string(),
            ":ID: 2025-05-01T12:00:00+0100".to_string(),
            ":TAGS: test multiline".to_string(),
            ":END:".to_string(),
            "".to_string(),
            "First line of content".to_string(),
            "Second line with *formatting*".to_string(),
            "Third line".to_string(),
        ];
        
        let post = Post::from(post_lines);

        assert_eq!(post.id(), "2025-05-01T12:00:00+0100");
        assert_eq!(post.tags(), &Some(vec!["test".to_string(), "multiline".to_string()]));
        
        let content = post.content();
        assert_eq!(content.lines().count(), 3);
        assert!(content.contains("First line of content"));
        assert!(content.contains("Second line with *formatting*"));
        assert!(content.contains("Third line"));

        #[cfg(feature = "autotokenize")] {
            assert!(!post.tokens().is_empty());
            assert!(post.tokens().iter().any(|t| matches!(t, Token::Bold(_))));
        }
        
    }
}
