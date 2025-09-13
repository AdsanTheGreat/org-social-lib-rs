# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to (as crates are supposed to) [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- **Feed/Notifications/Threading refactor:**
  - Feed now holds all posts, profiles, and a post-to-profile map using `Arc<Profile>` for safe shared ownership
  - NotificationFeed and ThreadView now hold references to their base Feed and use it for all post/profile lookups and construction
  - Added `Feed::profile_from_post` for mapping from a post reference to its profile
  - Notification and threading logic now operate on posts from Feed, not raw vectors. They also hold refs to the feeds used in creation

### Technical Details
- Post now implements Eq, PartialEq and Hash - only the id is considered for those traits
- Profile now implements Eq, PartialEq and Hash - only the title & nick are considered for those traits

## [0.4.3] - 10-09-2025
### Fixed
- **Post summary**: Fixed the `Post::summary` function panicking when the split is in the middle of a multi-byte character (e.g. emoji)

## [0.4.2] - 9-09-2025
### Changed
- **Post parsing**: Empty properties in posts are treated as None, not Some("")

## [0.4.1] - 9-09-2025
### Added
- **NewPostState struct**: `NewPostState` now implements clone and debug traits

## [0.4.0] - 9-09-2025

### Removed
- **Removed** the `reply` module and all associated structs and enums
- **New Post Module**: Removed the "editor" functionality completely - clients should implement their own
  - Removed `NewPostField` enum and all its variants
  - Removed `NewPostManager` struct and its methods
  - Removed functions dealing with modifying the content
  - Removed fields related to cursor position

### Changed
- **Moved** the post's file saving logic to the Post struct
- **Thread view**: now sorts by latest activity time in a thread, not the root post time
- **New Post to Post**: `NewPostManager::create_new_post()` is now `NewPostManager::create_post()`
  - Takes in the client name to set on the post
  - Returns a `Post` now
- **Automatic post tokenization**: Automatic post content parsing is now locked behind a non-default feature flag
  - New `autotokenize` feature flag to call content parsing upon post creation and content modification
  - If the feature is disabled, content modification will clear tokens and blocks - to not have them be outdated
  - Manual parsing will have to be called explicitly if the feature is disabled
  
### Added
- **Post types: ** Added `PostType` enum to classify posts into types
  - Regular, Reply, Reaction, Poll, PollVote, SimplePollVote
  - `Post::post_type()` method to get the type of a post
- **Thread view**: now has a method to insert a post into the tree after the fact
  - Expensive operation for replies, as the tree has to be searched recursively for the parent post
- **New Post Expansion**: It now has helpers and fields to also be a reply or a vote
- `Post` has a helper function to summarize it's content as first n chars - due to lack of title structure

## [0.3.1] - 3-09-2025
### Fixed
- **Bug Fix**: Corrected `update_poll_node` method signature
  - Changed `&mut self` to `&self`, it does not need to be mutable

## [0.3.0] - 3-09-2025

### Added
- **Poll Feature**: Support for polls in posts
  - Added `poll` module with `Poll`, `PollOption`, and `PollStatus` types
  - Poll detection: Posts with `poll_end` timestamp and "- [ ]" options
  - Poll parsing: Extract poll options from post content
  - Poll voting: Create reply posts with `poll_option` field to vote
  - Poll results: Count votes and calculate percentages from replies
  - Poll status tracking: Automatic active/ended status based on timestamps
- **Blocks Integration**: Extended activatable elements to include polls
  - Added `Poll` variant to `ActivatableElement` enum
  - Poll blocks can be collapsed/expanded like other content blocks
  - Integrated poll parsing into `parse_blocks_with_poll_end` function
- **Reply System Enhancement**: Extended reply functionality for poll voting
  - Added `PollOption` field to `ReplyField` enum
  - Poll vote creation through reply system
  - Helper function `create_poll_vote` for easy vote replies
- **Post Enhancements**: Added poll-related methods to Post struct
  - `is_poll()`: Check if post contains a poll
  - `get_poll()`: Extract poll data from post
  - `is_poll_vote()`: Check if post is a poll vote
  - Enhanced content parsing to include poll detection
- **Thread integration**: Integrated poll updates into thread management
  - Added `update_poll_node` method to `ThreadNode` for updating polls
- **Mention Enhancements**: Added methods to enable getting from a nickname to an org-social mention
  - Added `create_follow_map` method to `Profile` struct - creates a mapping of followed nicknames to their corresponding urls
  - Added `parse_followed_nickname_to_mention` method to `Profile` struct - serializes a nickname into a full org-social mention string

### Technical Details
- Poll options extracted from content lines starting with "- [ ]"
- Vote counting by analyzing replies with matching `poll_option` field

## [0.2.2] - 1-09-2025

### Added
- **Timeout Support**: Introduced optional timeout for feed fetching
  - Added `timeout` parameter to `get_feeds` and `get_feeds_from_profile` functions
  - Default timeout of 30 seconds applied in convenience functions

## [0.2.1] - 31-08-2025

### Changed
- **Tokenizer Refactoring**: Reduced duplication in how formatting tokens are parsed
  - Extracted common formatting parsing logic into reusable generic functions
  - All formatting types are now consistent

### Improved
- **Test Optimization**: Updates to test suite
  - Reduced test count
  - Refreshed the tests for more coverage of new features
  - Removed compiler warnings about tests in blocks.rs

### Technical Details
- Generic `parse_delimited_text_multi()` function can handle both single and multi-character delimiters
- Zero functional changes - all existing behavior preserved

## [0.2.0] - 28-08-2025

### Added
- **New tokens**: Introduced new org-mode token
  - Added `mention` token, for org-social specific user mention. Takes priority over the link token.
  - Added `underline` token support
  - Added `strikethrough` token support
- **Post Content Parsing**: Posts now automatically parse their content into tokens and blocks
  - Added `tokens` field to `Post` struct containing parsed tokens
  - Added `blocks` field to `Post` struct containing parsed blocks
  - Added `parse_content()` method for manual content parsing
  - Added `tokens()` getter method to access parsed tokens
  - Added `blocks()` getter method to access parsed blocks
- **Notifications System**: Notification summary of replies and mentions, organized the same way feed is.
  - Added `notifications` module with full notification handling
  - Added `NotificationFeed` struct for aggregating and managing notifications
  - Added `Notification` struct to wrap posts with notification context
  - Added `NotificationType` enum (Mention, Reply, MentionAndReply)
  - Automatic detection of posts that mention users via tokenized mentions, reply_to field or "@username" in content

### Changed
- **Enhanced Post Creation**: `Post::new()` now automatically parses content into tokens and blocks
- **Automatic Content Updates**: `set_content()` method now automatically re-parses content when modified

### Technical Details
- Posts now hold both raw content and structured representations (tokens/blocks)
- Content parsing is performed automatically on post creation and content modification
- Minor change in how unclosed tokens are handled
- Notifications are deduplicated - a reply that also mentions appears only once.


### Tests Added
- **Notification Tests**: 
  - Test for notification creation from mentions
  - Test for notification creation from replies
  - Test for notification deduplication
  - Test for feed creation
- **Post parsing Tests**: 
  - Test for automatic content parsing on post creation
  - Test for multi-line content preservation
  - Test for content re-parsing when modified via `set_content()`
  - Test for parsing posts from org-social format with multi-line content
  - Test for org-mode block parsing within post content
- **Notifications Tests**: 
  - Test for mention detection in posts via tokenized mentions
  - Test for reply detection via reply_to field matching
  - Test for notification feed creation and aggregation
  - Test for deduplication of posts that both mention and reply
