//! Parser module for org-social file format.
//! 
//! This module provides functionality to parse and serialize complete
//! org-social files containing profiles and posts.

// Re-export types for backward compatibility
pub use crate::profile::Profile;
pub use crate::post::Post;

/// Parse an org-social formatted file into a profile and list of posts.
///
/// # Arguments
/// 
/// * `file_content` - The raw content of the org-social file
/// * `source` - Optional source identifier to be associated with posts
///
/// # Returns
/// 
/// A tuple containing the parsed profile and a vector of posts.
pub fn parse_file(file_content: &str, source: Option<String>) -> (Profile, Vec<Post>) {
    let lines = file_content
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();
    let mut posts = Vec::new();

    // Find the start of the posts section
    let posts_index = lines
        .iter()
        .position(|line| line.starts_with("* Posts"))
        .unwrap_or(lines.len());

    // Parse the profile section (everything before "* Posts")
    let profile_section_lines = lines.split_at(posts_index).0.to_vec();
    let mut profile = Profile::from(profile_section_lines);
    profile.set_source(source.clone());

    // Parse the posts section if it exists
    if posts_index < lines.len() {
        let posts_section_lines = lines.split_at(posts_index + 1).1.to_vec();

        // Find all post start indices (lines beginning with "**")
        let mut post_indices = Vec::new();
        for (i, line) in posts_section_lines.iter().enumerate() {
            if line.starts_with("**") {
                post_indices.push(i);
            }
        }

        // Parse each individual post
        for (i, &start_index) in post_indices.iter().enumerate() {
            let end_index = if i + 1 < post_indices.len() {
                post_indices[i + 1]
            } else {
                posts_section_lines.len()
            };

            let post_lines = posts_section_lines[start_index..end_index].to_vec();
            if !post_lines.is_empty() {
                let mut post = Post::from(post_lines);
                post.set_source(source.clone());
                posts.push(post);
            }
        }
    }

    (profile, posts)
}

/// Serialize a profile and posts back to org-social format.
/// 
/// Creates a complete org-social file with profile metadata and posts section.
/// 
/// # Arguments
/// 
/// * `profile` - The profile to serialize
/// * `posts` - The vector of posts to serialize
/// 
/// # Returns
/// 
/// A string containing the complete org-social file content.
pub fn serialize_file(profile: &Profile, posts: &[Post]) -> String {
    let mut output = Vec::new();

    // Add profile metadata
    let profile_content = profile.to_org_social();
    if !profile_content.is_empty() {
        output.push(profile_content);
        output.push("".to_string()); // Empty line after profile
    }

    // Add posts section
    if !posts.is_empty() {
        output.push("* Posts".to_string());
        
        for post in posts {
            output.push(post.to_org_social());
            output.push("".to_string()); // Empty line between posts
        }
        
        // Remove the last empty line
        if output.last() == Some(&"".to_string()) {
            output.pop();
        }
    }

    output.join("\n")
}