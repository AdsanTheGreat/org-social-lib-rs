//! Profile module for org-social file format.
//! 
//! This module contains the Profile struct and its implementations
//! for parsing and serializing user profile metadata.

/// Represents a user profile parsed from an org-social file.
/// 
/// Contains metadata about the user.
#[derive(Clone)]
#[derive(Default)]
pub struct Profile {
    title: String,
    nick: String,
    description: String,
    avatar: Option<String>,
    link: Option<Vec<String>>,
    follow: Option<Vec<(String, String)>>,
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
                        // Parse "nick url" format
                        let follow_parts: Vec<&str> = parts[1].trim().splitn(2, ' ').collect();
                        if follow_parts.len() == 2 {
                            follow.as_mut().unwrap().push((
                                follow_parts[0].to_string(),
                                follow_parts[1].to_string(),
                            ));
                        }
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

impl Profile {
    pub fn follow(&self) -> &Option<Vec<(String, String)>> {
        &self.follow
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

        if let Some(contacts) = &self.contact {
            for contact in contacts {
                lines.push(format!("#+CONTACT: {contact}"));
            }
        }

        lines.join("\n")
    }
}
