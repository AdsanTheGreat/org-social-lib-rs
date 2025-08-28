//! # org-social-lib-rs
//!
//! A Rust library for parsing and interacting with [Org-social](https://github.com/tanrax/org-social) decentralized social networks.
//!
//! ## Overview
//!
//! org-social-lib-rs provides the core functionality for working with org-social feeds. It handles parsing org-mode social files, 
//! tokenizing different org mode elements, and managing using the social network. It basically constitutes the backend of any org-social application.
//!
//! ## Features
//!
//! - **Org-social Parsing**: Parse org-social files into profiles and posts
//! - **Network Fetching**: Asynchronous fetching of remote org-social feeds
//! - **Threading System**: Build threaded conversation view from reply relationships
//! - **Feed Aggregation**: Combine multiple feeds into a unified, chronologically sorted feed
//! - **Post Management**: Create, parse, and manage social posts with metadata
//! - **Reply Handling**: Parse and create replies between posts
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use org_social_lib_rs::parser;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Parse a local org-social file
//!     let content = std::fs::read_to_string("social.org")?;
//!     let (profile, posts) = parser::parse_file(&content, Some("https://example.com/social.org".to_string()));
//!
//!     // Access posts and profile information
//!     println!("Feed author: {}", profile.title());
//!     for post in posts {
//!         println!("Post: {} - {}", post.id(), post.content());
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod blocks;
pub mod feed;
pub mod network;
pub mod new_post;
pub mod parser;
pub mod post;
pub mod profile;
pub mod reply;
pub mod threading;
pub mod tokenizer;
pub mod util;
