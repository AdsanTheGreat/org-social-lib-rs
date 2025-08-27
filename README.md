# org-social-lib-rs

A Rust library for parsing and interacting with [Org-social](https://github.com/tanrax/org-social) decentralized social networks.

Current version is targeting 1.1 release.

## Overview

org-social-lib-rs provides the core functionality for working with org-social feeds. It handles parsing org-mode social files, tokenizing different org mode elements (WIP), and managing using the social network.
It basically constitutes the backend of any org-social application.

## Features

- **Org-social Parsing**: Parse org-social files into profiles and posts
- **Network Fetching**: Asynchronous fetching of remote org-social feeds
- **Threading System**: Build threaded conversation view from reply relationships
- **Feed Aggregation**: Combine multiple feeds into a unified, chronologically sorted feed
- **Post Management**: Create, parse, and manage social posts with metadata
- **Reply Handling**: Parse and create replies between posts

### File Format

org-social-rs works with org-social files as-specified in current target release, with some exceptions where I've noticed people differ. The parsing is very unstable, it will probably explode if there are any major changes.

## TODO
In no particular order:
- Add missing org-mode features:
  - Code block syntax highlighting
  - Embeds
  - Tables
  - Latex - maybe
  - Mentions
  - Lists
  - Polls
- Network exploration - view not followed users
- Gathering notifications - mentions & replies
- Documentation
- Publish on crates.io

## Installation

As of now, the crate is not on crates.io yet. As such, it has to be pulled in manually through git.
To use the crate, add this to your `Cargo.toml`:

```toml
[dependencies]
org-social-lib-rs = { git = "https://github.com/AdsanTheGreat/org-social-lib-rs" }
```

## Quick Start

```rust
use org_social_lib_rs::{parser, feed, network};

// Parse a local org-social file
let content = std::fs::read_to_string("social.org")?;
let (profile, posts) = parser::parse_file(&content, "https://example.com/social.org")?;

// Create a feed from multiple sources
let client = reqwest::Client::new();
let follow_urls = vec!["https://friend1.com/social.org", "https://friend2.com/social.org"];
let combined_feed = feed::Feed::create_combined_feed(&follow_urls, &client).await?;

// Access posts and profile information
println!("Feed author: {}", profile.title);
for post in posts {
    println!("Post: {} - {}", post.id, post.content);
}
```

## Documentation

Currently, the only documentation are the documenting comments. They are very WIP, inconsitent and usually not enough. In near future, it is to be expanded into a proper crate documentation.




## Applications Using This Library

- [org-social-rs](https://github.com/AdsanTheGreat/org-social-rs) - CLI and TUI client

## Contributing

Report issues (there are probably a lot of them), submit pull requests, help is welcome.

## License

This project is licensed under the GNU General Public License v3.0.

## Related Projects

- [org-social.el](https://github.com/tanrax/org-social.el) - Original Emacs client  
- [org-social](https://github.com/tanrax/org-social) - Protocol specification
