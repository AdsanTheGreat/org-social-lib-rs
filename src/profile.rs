//! Profile module for org-social file format.
//! 
//! This module contains the Profile struct and its implementations
//! for parsing and serializing user profile metadata.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Represents a user profile parsed from an org-social file.
/// 
/// Contains metadata about the user.
#[derive(Clone, Default)]
pub struct Profile {
    title: String,
    nick: String,
    description: String,
    avatar: Option<String>,
    link: Option<Vec<String>>,
    follow: Option<Vec<(String, String)>>,
    groups: Option<Vec<(String, String)>>,
    contact: Option<Vec<String>>,
    source: Option<String>,
}


impl From<&Profile> for Profile {
    fn from(profile: &Profile) -> Self {
        Profile {
            title: profile.title.clone(),
            nick: profile.nick.clone(),
            description: profile.description.clone(),
            avatar: profile.avatar.clone(),
            link: profile.link.clone(),
            follow: profile.follow.clone(),
            groups: profile.groups.clone(),
            contact: profile.contact.clone(),
            source: profile.source.clone(),
        }
    }
}

impl From<Vec<String>> for Profile {
    /// Parse a profile from org-mode formatted lines.
    /// 
    /// Extracts profile metadata from a social.org file
    fn from(profile_section_lines: Vec<String>) -> Self {
        let mut title = String::new();
        let mut nick = String::new();
        let mut description = String::new();
        let mut avatar: Option<String> = None;
        let mut link: Option<Vec<String>> = None;
        let mut follow: Option<Vec<(String, String)>> = None;
        let mut groups: Option<Vec<(String, String)>> = None;
        let mut contact: Option<Vec<String>> = None;

        for line in profile_section_lines {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                match parts[0].trim() {
                    "#+TITLE" => title = parts[1].trim().to_string(),
                    "#+NICK" => nick = parts[1].trim().to_string(),
                    "#+DESCRIPTION" => description = parts[1].trim().to_string(),
                    "#+AVATAR" => avatar = Some(parts[1].trim().to_string()),
                    "#+LINK" => {
                        if link.is_none() {
                            link = Some(Vec::new());
                        }
                        link.as_mut().unwrap().push(parts[1].trim().to_string());
                    }
                    "#+FOLLOW" => {
                        if follow.is_none() {
                            follow = Some(Vec::new());
                        }
                        // Parse "nick url" format or "url"
                        let follow_parts: Vec<&str> = parts[1].trim().splitn(2, ' ').collect();
                        if follow_parts.len() == 2 {
                            follow.as_mut().unwrap().push((
                                follow_parts[0].to_string(),
                                follow_parts[1].to_string(),
                            ));
                        } else if follow_parts.len() == 1 {
                            // If only url is provided, use empty string for nick, as per:
                            // https://github.com/tanrax/org-social/issues/18#issuecomment-3245769906
                            follow.as_mut().unwrap().push((
                                String::new(),
                                follow_parts[0].to_string(),
                            ));
                        }
                    }
                    "#+GROUP" => {
                        if groups.is_none() {
                            groups = Some(Vec::new());
                        }
                        // Parse "name url" format or "url"
                        let groups_parts: Vec<&str> = parts[1].trim().splitn(2, ' ').collect();
                        if groups_parts.len() == 2 {
                            groups.as_mut().unwrap().push((
                                groups_parts[0].to_string(),
                                groups_parts[1].to_string(),
                            ));
                        } // No unnamed groupss
                    }
                    "#+CONTACT" => {
                        if contact.is_none() {
                            contact = Some(Vec::new());
                        }
                        contact.as_mut().unwrap().push(parts[1].trim().to_string());
                    }
                    _ => {}
                }
            }
        }
        
        Profile {
            title,
            nick,
            description,
            avatar,
            link,
            follow,
            groups,
            contact,
            source: None,
        }
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = Vec::new();
        
        output.push(format!("Title: {}", self.title));
        output.push(format!("Nick: {}", self.nick));
        
        if !self.description.is_empty() {
            output.push(format!("Description: {}", self.description));
        }
        
        // At some point maybe render a tiny version of it in the terminal? (depending on the terminal capabilities)
        if let Some(avatar) = &self.avatar {
            output.push(format!("Avatar: {}", avatar));
        }
        
        if let Some(links) = &self.link {
            if !links.is_empty() {
                if links.len() == 1 {
                    output.push(format!("Link: {}", links[0]));
                } else {
                    output.push(format!("Links:"));
                    for (i, link) in links.iter().enumerate() {
                        output.push(format!("  {}. {}", i + 1, link));
                    }
                }
            }
        }
        
        if let Some(follows) = &self.follow {
            if !follows.is_empty() {
                output.push(format!("Following: {} {}", 
                    follows.len(),
                    if follows.len() == 1 { "user" } else { "users" }
                ));
                for (i, (name, url)) in follows.iter().enumerate() {
                    output.push(format!("  {}. {} - {}", 
                        i + 1,
                        name, 
                        url
                    ));
                }
            }
        }
        
        if let Some(groups) = &self.groups {
            if !groups.is_empty() {
                output.push(format!("groups: {} {}", 
                    groups.len(),
                    if groups.len() == 1 { "group" } else { "groups" }
                ));
                for (i, (name, url)) in groups.iter().enumerate() {
                    output.push(format!("  {}. {} - {}", 
                        i + 1,
                        name, 
                        url
                    ));
                }
            }
        }
        
        if let Some(contacts) = &self.contact {
            if !contacts.is_empty() {
                if contacts.len() == 1 {
                    output.push(format!("Contact: {}", contacts[0]));
                } else {
                    output.push(format!("Contact:"));
                    for (i, contact) in contacts.iter().enumerate() {
                        output.push(format!("  {}. {}", i + 1, contact));
                    }
                }
            }
        }
        
        if let Some(source) = &self.source {
            output.push(format!("Source: {}", source));
        }
        
        write!(f, "{}", output.join("\n"))
    }
}

impl PartialEq for Profile {
    /// Profiles are considered equal when they have the same title and nick
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title &&
        self.nick == other.nick
    }
}

impl Eq for Profile {}

impl core::hash::Hash for Profile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.title.hash(state);
        self.nick.hash(state);
    }
}

impl Profile {
    pub fn follow(&self) -> &Option<Vec<(String, String)>> {
        &self.follow
    }

    pub fn groups(&self) -> &Option<Vec<(String, String)>> {
        &self.groups
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn nick(&self) -> &str {
        &self.nick
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn avatar(&self) -> Option<&String> {
        self.avatar.as_ref()
    }

    pub fn link(&self) -> Option<&Vec<String>> {
        self.link.as_ref()
    }

    pub fn contact(&self) -> Option<&Vec<String>> {
        self.contact.as_ref()
    }

    pub fn source(&self) -> Option<&String> {
        self.source.as_ref()
    }

    pub fn set_source(&mut self, source: Option<String>) {
        self.source = source;
    }

    pub fn set_nick(&mut self, nick: String) {
        self.nick = nick;
    }

    pub fn to_org_social(&self) -> String {
        let mut lines = Vec::new();

        // Add required fields
        if !self.title.is_empty() {
            lines.push(format!("#+TITLE: {}", self.title));
        }
        if !self.nick.is_empty() {
            lines.push(format!("#+NICK: {}", self.nick));
        }
        if !self.description.is_empty() {
            lines.push(format!("#+DESCRIPTION: {}", self.description));
        }

        // Add optional fields
        if let Some(avatar) = &self.avatar {
            lines.push(format!("#+AVATAR: {avatar}"));
        }

        if let Some(links) = &self.link {
            for link in links {
                lines.push(format!("#+LINK: {link}"));
            }
        }

        if let Some(follows) = &self.follow {
            for (nick, url) in follows {
                lines.push(format!("#+FOLLOW: {nick} {url}"));
            }
        }

        if let Some(groups) = &self.groups {
            for (name, url) in groups {
                lines.push(format!("#+GROUP: {name} {url}"));
            }
        }

        if let Some(contacts) = &self.contact {
            for contact in contacts {
                lines.push(format!("#+CONTACT: {contact}"));
            }
        }

        lines.join("\n")
    }

    pub fn add_follow(&mut self, nick: String, url: String) {
        if self.follow.is_none() {
            self.follow = Some(Vec::new());
        }
        if let Some(follow) = &mut self.follow {
            follow.push((nick, url));
        }
    }

    pub fn create_follow_map(&self) -> HashMap<String, String> {
        let mut follow_map = HashMap::new();
        if let Some(follows) = &self.follow {
            for (name, url) in follows {
                follow_map.insert(name.clone(), url.clone());
            }
        }
        follow_map
    }

    /// Parses a followed user's nickname to an org-social mention syntax
    pub fn parse_followed_nickname_to_mention(&self, text: &str) -> Option<String> {
        let follow_map = self.create_follow_map();
        let mut result = text.to_string();
        if result.starts_with('@') {
            result = result.trim_start_matches('@').to_string();
        }

        if let Some(url) = follow_map.get(&result) {
            result = format!("[[org-social:{}][@{}]]", url, result);
        } else {
            return None;
        }

        Some(result)
    }

    /// Save this profile to a file, preserving comments, unknown lines, and posts section.
    /// 
    /// This function reads the existing file, identifies the profile section (everything before "* Posts"),
    /// replaces only the known profile lines while preserving comments and unknown syntax in their
    /// original positions, and keeps the posts section intact.
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - Path to the org-social file to update
    /// 
    /// # Returns
    /// 
    /// Result indicating success or failure with error details
    /// 
    /// # Errors
    /// 
    /// Returns an error if the file cannot be read or written, or if I/O operations fail.
    pub fn save_to_file<P: AsRef<Path>>(&self, file_path: P) -> io::Result<()> {
        let file_path = file_path.as_ref();
        
        // Read existing file content, or start with empty if file doesn't exist
        let existing_content = if file_path.exists() {
            fs::read_to_string(file_path)?
        } else {
            String::new()
        };
        let lines: Vec<String> = existing_content
            .lines()
            .map(String::from)
            .collect();

        // Split into profile section and posts section
        let posts_index = lines
            .iter()
            .position(|line| line.starts_with("* Posts"))
            .unwrap_or(lines.len());
        let profile_section_lines = lines.split_at(posts_index).0;
        let posts_section_lines = if posts_index < lines.len() {
            lines.split_at(posts_index).1
        } else {
            &[]
        };

        let known_profile_prefixes = [
            "#+TITLE:",
            "#+NICK:",
            "#+DESCRIPTION:",
            "#+AVATAR:",
            "#+LINK:",
            "#+FOLLOW:",
            "#+GROUP:",
            "#+CONTACT:",
        ];

        // Create a set of indices for lines that should be skipped (known profile lines)
        let mut skip_indices = std::collections::HashSet::new();
        for (i, line) in profile_section_lines.iter().enumerate() {
            let is_known_profile_line = known_profile_prefixes
                .iter()
                .any(|prefix| line.trim().starts_with(prefix));
            
            if is_known_profile_line {
                skip_indices.insert(i);
            }
        }

        let mut output = Vec::new();

        // Add profile section lines, preserving positions but skipping known profile lines
        for (i, line) in profile_section_lines.iter().enumerate() {
            if !skip_indices.contains(&i) {
                output.push(line.clone());
            }
        }

        // Find a good position to insert the new profile content
        // We'll add it after any preserved lines but before posts
        let profile_content = self.to_org_social();
        if !profile_content.is_empty() {
            // Add an empty line before profile if there were preserved lines
            if !output.is_empty() && !output.last().unwrap().trim().is_empty() {
                output.push("".to_string());
            }
            output.push(profile_content.clone());
        }

        // Add posts section if it existed, or create one if it didn't
        if !posts_section_lines.is_empty() {
            // Add empty line before posts section if profile content was added
            if !profile_content.is_empty() {
                output.push("".to_string());
            }
            for line in posts_section_lines {
                output.push(line.clone());
            }
        } else {
            // No posts section existed, so create one in preparation for future posts
            if !profile_content.is_empty() {
                output.push("".to_string());
            }
            output.push("* Posts".to_string());
        }

        // Write the new content to the file
        let final_content = output.join("\n");
        fs::write(file_path, final_content)?;

        Ok(())
    }
}
