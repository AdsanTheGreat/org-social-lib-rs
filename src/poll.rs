//! Poll module for org-social posts.
//!
//! This module provides functionality to parse, manage, and analyze polls
//! within org-social posts.

use chrono::{FixedOffset, Utc};
use crate::post::Post;
use crate::util;

/// Represents a poll option with its text and vote count
#[derive(Debug, Clone, PartialEq)]
pub struct PollOption {
    pub text: String,
    pub votes: usize,
}

/// Represents a poll's current status
#[derive(Debug, Clone, PartialEq)]
pub enum PollStatus {
    Active,
    Ended,
    Invalid,
}

/// Represents a complete poll with options, metadata, and results
#[derive(Debug, Clone, PartialEq)]
pub struct Poll {
    pub options: Vec<PollOption>,
    pub poll_end: Option<String>,
    pub status: PollStatus,
    pub total_votes: usize,
    pub start_line: usize,
    pub end_line: usize,
}

impl Poll {
    /// Create a new poll from parsed options and poll_end timestamp
    pub fn new(options: Vec<String>, poll_end: Option<String>, start_line: usize, end_line: usize) -> Self {
        let poll_options: Vec<PollOption> = options
            .into_iter()
            .map(|text| PollOption { text, votes: 0 })
            .collect();

        let status = Self::determine_status(&poll_end);

        Poll {
            options: poll_options,
            poll_end,
            status,
            total_votes: 0,
            start_line,
            end_line,
        }
    }

    /// Determine if the poll is active, ended, or invalid based on poll_end timestamp
    fn determine_status(poll_end: &Option<String>) -> PollStatus {
        match poll_end {
            Some(end_time) => {
                match util::parse_timestamp(end_time) {
                    Ok(end_dt) => {
                        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                        if now > end_dt {
                            PollStatus::Ended
                        } else {
                            PollStatus::Active
                        }
                    },
                    Err(_) => PollStatus::Invalid,
                }
            },
            None => PollStatus::Invalid,
        }
    }

    /// Update poll status based on current time
    pub fn update_status(&mut self) {
        self.status = Self::determine_status(&self.poll_end);
    }

    /// Add a vote to a specific option by index
    pub fn add_vote(&mut self, option_index: usize) -> bool {
        if option_index < self.options.len() {
            self.options[option_index].votes += 1;
            self.total_votes += 1;
            true
        } else {
            false
        }
    }

    /// Add a vote by option text (case-insensitive)
    pub fn add_vote_by_text(&mut self, option_text: &str) -> bool {
        let option_text_lower = option_text.trim().to_lowercase();
        for option in &mut self.options {
            if option.text.trim().to_lowercase() == option_text_lower {
                option.votes += 1;
                self.total_votes += 1;
                return true;
            }
        }
        false
    }

    pub fn add_vote_from_reply(&mut self, reply: &Post) -> bool {
        if let Some(poll_option) = reply.poll_option() {
            if self.options.iter().any(|opt| opt.text == *poll_option) {
                return self.add_vote_by_text(poll_option);
            }
        }
        false
    }

    /// Get poll results as percentages
    pub fn get_results(&self) -> Vec<(String, usize, f32)> {
        self.options
            .iter()
            .map(|option| {
                let percentage = if self.total_votes > 0 {
                    (option.votes as f32 / self.total_votes as f32) * 100.0
                } else {
                    0.0
                };
                (option.text.clone(), option.votes, percentage)
            })
            .collect()
    }

    /// Check if the poll is still accepting votes
    pub fn is_active(&self) -> bool {
        matches!(self.status, PollStatus::Active)
    }

    /// Get a summary string for display
    pub fn get_summary(&self) -> String {
        let status_str = match self.status {
            PollStatus::Active => "Active",
            PollStatus::Ended => "Ended",
            PollStatus::Invalid => "Invalid",
        };
        
        format!("Poll ({} options, {} votes, {})", 
                self.options.len(), 
                self.total_votes, 
                status_str)
    }

    pub fn clear_votes(&mut self) {
        for option in &mut self.options {
            option.votes = 0;
        }
        self.total_votes = 0;
    }
}

/// Detect if a post contains a poll
pub fn is_poll_post(post: &Post) -> bool {
    // Must have poll_end set
    if post.poll_end().is_none() {
        return false;
    }

    // Must contain poll options in content
    has_poll_options_in_content(post.content())
}

/// Check if content contains poll options (consecutive lines starting with "- [ ]")
fn has_poll_options_in_content(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut consecutive_options = 0;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]") {
            consecutive_options += 1;
        } else if consecutive_options > 0 {
            // Found at least one option before, so we have a poll
            return consecutive_options >= 2; // Require at least 2 options
        }
    }

    // Check if we ended with options
    consecutive_options >= 2
}

/// Parse poll options from post content
pub fn parse_poll_from_content(content: &str, poll_end: Option<String>) -> Option<Poll> {
    let lines: Vec<&str> = content.lines().collect();
    let mut poll_options = Vec::new();
    let mut start_line = None;
    let mut end_line = None;
    let mut in_poll_section = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("- [ ]") {
            if !in_poll_section {
                in_poll_section = true;
                start_line = Some(line_idx);
            }
            
            // Extract option text after "- [ ]"
            let option_text = trimmed[5..].trim().to_string();
            if !option_text.is_empty() {
                poll_options.push(option_text);
            }
            end_line = Some(line_idx);
        } else if in_poll_section && !trimmed.is_empty() {
            // Non-empty line after poll options ends the poll section
            break;
        }
    }

    // Need at least 2 options to be a valid poll
    if poll_options.len() >= 2 {
        Some(Poll::new(
            poll_options,
            poll_end,
            start_line.unwrap_or(0),
            end_line.unwrap_or(0),
        ))
    } else {
        None
    }
}

/// Parse poll from a Post
pub fn parse_poll_from_post(post: &Post) -> Option<Poll> {
    if !is_poll_post(post) {
        return None;
    }

    parse_poll_from_content(post.content(), post.poll_end().clone())
}

/// Count votes for a poll from reply posts
pub fn count_poll_votes(poll_post: &Post, replies: &[Post]) -> Option<Poll> {
    let mut poll = parse_poll_from_post(poll_post)?;

    // Count votes from replies that have poll_option set
    for reply in replies {
        if let Some(vote_option) = reply.poll_option() {
            poll.add_vote_by_text(vote_option);
        }
    }

    Some(poll)
}

/// Create a vote reply post for a poll option
pub fn create_vote_reply(poll_post_id: &str, option_text: &str, voter_content: Option<&str>) -> Post {
    let timestamp = util::get_current_timestamp();
    let content = voter_content.unwrap_or("").to_string();
    
    let mut reply_post = Post::new(timestamp, content);
    reply_post.set_reply_to(Some(poll_post_id.to_string()));
    reply_post.set_poll_option(Some(option_text.to_string()));
    reply_post.set_client(Some("org-social-rs".to_string()));
    
    reply_post
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_option_creation() {
        let option = PollOption {
            text: "Option A".to_string(),
            votes: 5,
        };
        assert_eq!(option.text, "Option A");
        assert_eq!(option.votes, 5);
    }

    #[test]
    fn test_poll_status_determination() {
        // Test active poll (future timestamp)
        let future_time = "2030-01-01T12:00:00+00:00".to_string();
        let status = Poll::determine_status(&Some(future_time));
        assert_eq!(status, PollStatus::Active);

        // Test ended poll (past timestamp)
        let past_time = "2020-01-01T12:00:00+00:00".to_string();
        let status = Poll::determine_status(&Some(past_time));
        assert_eq!(status, PollStatus::Ended);

        // Test invalid poll (no timestamp)
        let status = Poll::determine_status(&None);
        assert_eq!(status, PollStatus::Invalid);
    }

    #[test]
    fn test_poll_creation() {
        let options = vec!["Option A".to_string(), "Option B".to_string()];
        let poll_end = Some("2030-01-01T12:00:00+00:00".to_string());
        let poll = Poll::new(options, poll_end, 0, 2);

        assert_eq!(poll.options.len(), 2);
        assert_eq!(poll.total_votes, 0);
        assert_eq!(poll.status, PollStatus::Active);
    }

    #[test]
    fn test_add_vote() {
        let options = vec!["Option A".to_string(), "Option B".to_string()];
        let poll_end = Some("2030-01-01T12:00:00+00:00".to_string());
        let mut poll = Poll::new(options, poll_end, 0, 2);

        assert!(poll.add_vote(0));
        assert_eq!(poll.options[0].votes, 1);
        assert_eq!(poll.total_votes, 1);

        assert!(!poll.add_vote(5)); // Invalid index
        assert_eq!(poll.total_votes, 1);
    }

    #[test]
    fn test_add_vote_by_text() {
        let options = vec!["Option A".to_string(), "Option B".to_string()];
        let poll_end = Some("2030-01-01T12:00:00+00:00".to_string());
        let mut poll = Poll::new(options, poll_end, 0, 2);

        assert!(poll.add_vote_by_text("Option A"));
        assert_eq!(poll.options[0].votes, 1);
        assert_eq!(poll.total_votes, 1);

        assert!(poll.add_vote_by_text("option b")); // Case insensitive
        assert_eq!(poll.options[1].votes, 1);
        assert_eq!(poll.total_votes, 2);

        assert!(!poll.add_vote_by_text("Option C")); // Non-existent option
        assert_eq!(poll.total_votes, 2);
    }

    #[test]
    fn test_poll_results() {
        let options = vec!["Option A".to_string(), "Option B".to_string()];
        let poll_end = Some("2030-01-01T12:00:00+00:00".to_string());
        let mut poll = Poll::new(options, poll_end, 0, 2);

        poll.add_vote(0);
        poll.add_vote(0);
        poll.add_vote(1);

        let results = poll.get_results();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "Option A");
        assert_eq!(results[0].1, 2);
        assert!((results[0].2 - 66.666664).abs() < 0.001); // Use approximate comparison for float
        assert_eq!(results[1].0, "Option B");
        assert_eq!(results[1].1, 1);
        assert!((results[1].2 - 33.333332).abs() < 0.001); // Use approximate comparison for float
    }

    #[test]
    fn test_has_poll_options_in_content() {
        let content_with_poll = "Here's a poll:\n- [ ] Option A\n- [ ] Option B\nWhat do you think?";
        assert!(has_poll_options_in_content(content_with_poll));

        let content_without_poll = "Just a regular post\nNo poll here";
        assert!(!has_poll_options_in_content(content_without_poll));

        let content_single_option = "Poll with one option:\n- [ ] Only option";
        assert!(!has_poll_options_in_content(content_single_option));
    }

    #[test]
    fn test_parse_poll_from_content() {
        let content = "What's your favorite color?\n- [ ] Red\n- [ ] Blue\n- [ ] Green\nThanks for voting!";
        let poll_end = Some("2030-01-01T12:00:00+00:00".to_string());
        
        let poll = parse_poll_from_content(content, poll_end).unwrap();
        assert_eq!(poll.options.len(), 3);
        assert_eq!(poll.options[0].text, "Red");
        assert_eq!(poll.options[1].text, "Blue");
        assert_eq!(poll.options[2].text, "Green");
        assert_eq!(poll.start_line, 1);
        assert_eq!(poll.end_line, 3);
    }

    #[test]
    fn test_is_poll_post() {
        let mut post = Post::new("test_id".to_string(), "- [ ] Option A\n- [ ] Option B".to_string());
        
        // Not a poll without poll_end
        assert!(!is_poll_post(&post));
        
        // Is a poll with poll_end
        post.set_poll_end(Some("2030-01-01T12:00:00+00:00".to_string()));
        assert!(is_poll_post(&post));
        
        // Not a poll without poll options in content
        post.set_content("Just regular content".to_string());
        assert!(!is_poll_post(&post));
    }

    #[test]
    fn test_create_vote_reply() {
        let reply = create_vote_reply("poll_post_id", "Option A", Some("I choose A!"));
        
        assert_eq!(reply.reply_to().as_deref(), Some("poll_post_id"));
        assert_eq!(reply.poll_option().as_deref(), Some("Option A"));
        assert_eq!(reply.content(), "I choose A!");
        assert_eq!(reply.client().as_deref(), Some("org-social-rs"));
    }

    #[test]
    fn test_count_poll_votes() {
        let mut poll_post = Post::new("poll_id".to_string(), "- [ ] Option A\n- [ ] Option B".to_string());
        poll_post.set_poll_end(Some("2030-01-01T12:00:00+00:00".to_string()));

        let reply1 = create_vote_reply("poll_id", "Option A", Some("Vote 1"));
        let reply2 = create_vote_reply("poll_id", "Option B", Some("Vote 2"));
        let reply3 = create_vote_reply("poll_id", "Option A", Some("Vote 3"));

        let replies = vec![reply1, reply2, reply3];
        let poll_with_votes = count_poll_votes(&poll_post, &replies).unwrap();

        assert_eq!(poll_with_votes.total_votes, 3);
        assert_eq!(poll_with_votes.options[0].votes, 2); // Option A
        assert_eq!(poll_with_votes.options[1].votes, 1); // Option B
    }
}
