# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to (as crates are supposed to) [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 1.09.2025

### Added
- **Timeout Support**: Introduced optional timeout for feed fetching
  - Added `timeout` parameter to `get_feeds` and `get_feeds_from_profile` functions
  - Default timeout of 30 seconds applied in convenience functions

## [0.2.1] - 31.08.2025

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

## [0.2.0] - 28.08.2025

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
