# org-social-lib-rs

A Rust library for parsing and interacting with [Org-social](https://github.com/tanrax/org-social) decentralized social networks.

Current version is targeting 1.2 release.

## Overview

org-social-lib-rs provides the core functionality for working with org-social feeds. It handles parsing org-mode social files, tokenizing different org mode elements (WIP), and managing using the social network.
It basically constitutes the backend of any org-social application.

## Features

- **Org-social Parsing**: Parse org-social files into profiles and posts
- **Network Fetching**: Asynchronous fetching of remote org-social feeds
- **Threading System**: Build threaded conversation view from reply relationships
- **Feed Aggregation**: Combine multiple feeds into a unified, chronologically sorted feed
- **Post Management**: Create, parse, and manage social posts with metadata
- **Notifications support**: Get the most important notifications for a user
- **Poll support**: Manage posts with polls

### File Format

org-social-rs works with org-social files as-specified in current target release, with some exceptions where I've noticed people differ. The parsing is very unstable, it is prone to breakage if the format changes.

## TODO
In no particular order:
- Add missing org-mode features:
  - Code block syntax highlighting
  - Embeds
  - Tables
  - Latex - maybe
  - Lists
- Network exploration - view not followed users
- Documentation

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
org-social-lib-rs = "0.4.0"
```

Or if you want to use the latest development version from git:

```toml
[dependencies]
org-social-lib-rs = { git = "https://github.com/AdsanTheGreat/org-social-lib-rs" }
```

### Optional Features
- `autotokenize` - Automatically parse post content into tokens and blocks when creating or modifying posts. Disabled by default for performance reasons (and it's not always needed to tokenize everything). If disabled, the `parse_content()` method has to be called manually to parse the content.

## Quick Start

```rust
use org_social_lib_rs::parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a local org-social file
    let content = std::fs::read_to_string("social.org")?;
    let (profile, posts) = parser::parse_file(&content, Some("https://example.com/social.org".to_string()));

    // Access posts and profile information
    println!("Feed author: {}", profile.title());
    for post in posts {
        println!("Post: {} - {}", post.id(), post.content());
    }
    
    Ok(())
}
```

## Documentation

Currently, the only documentation are the documenting comments. They are very WIP, inconsitent and usually not enough.

## Applications Using This Library

- [org-social-rs](https://github.com/AdsanTheGreat/org-social-rs) - CLI and TUI client

## Contributing

Report issues, submit pull requests, help is welcome.
You can also reach out at org-social itself.

## License

This project is licensed under the GNU General Public License v3.0.

## Related Projects

- [org-social.el](https://github.com/tanrax/org-social.el) - Original Emacs client  
- [org-social](https://github.com/tanrax/org-social) - Protocol specification
