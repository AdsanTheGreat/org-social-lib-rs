//! Relay API client for org-social relay servers.
//!
//! This module provides a client for communicating with Org Social Relay servers
//! and utilities for integrating relay data into feeds, notifications, and threads.

use std::collections::{HashMap, HashSet};
use std::fmt;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use urlencoding::encode;

use crate::network::{self, FetchOptions};
use crate::{
	feed::Feed,
	notifications::NotificationFeed,
	poll::{self, Poll},
	post::Post,
	profile::Profile,
	threading::ThreadView,
};

/// Generic API response wrapper returned by relay endpoints.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T, M = Value, L = Value> {
	pub r#type: String,
	pub errors: Vec<String>,
	pub data: T,
	#[serde(default)]
	pub meta: Option<M>,
	#[serde(rename = "_links", default)]
	pub links: Option<L>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RootInfo {
	pub name: String,
	pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MentionsMeta {
	pub feed: String,
	pub total: u64,
	pub version: String,
}

impl Default for MentionsMeta {
	fn default() -> Self {
		Self {
			feed: String::new(),
			total: 0,
			version: String::new(),
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReplyNode {
	pub post: String,
	#[serde(default)]
	pub children: Vec<ReplyNode>,
	#[serde(default)]
	pub moods: Vec<Mood>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepliesMeta {
	pub parent: String,
	pub version: String,
}

impl Default for RepliesMeta {
	fn default() -> Self {
		Self {
			parent: String::new(),
			version: String::new(),
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Mood {
	pub emoji: String,
	pub posts: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchMeta {
	pub version: String,
	pub query: String,
	pub total: u64,
	pub page: u32,
	#[serde(rename = "perPage")]
	pub per_page: u32,
	#[serde(rename = "hasNext")]
	pub has_next: bool,
	#[serde(rename = "hasPrevious")]
	pub has_previous: bool,
}

impl Default for SearchMeta {
	fn default() -> Self {
		Self {
			version: String::new(),
			query: String::new(),
			total: 0,
			page: 0,
			per_page: 0,
			has_next: false,
			has_previous: false,
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMessage {
	pub post: String,
	#[serde(default)]
	pub children: Vec<GroupMessage>,
	#[serde(default)]
	pub moods: Vec<Mood>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMessagesMeta {
	pub group: String,
	#[serde(default)]
	pub members: Vec<String>,
	pub version: String,
}

impl Default for GroupMessagesMeta {
	fn default() -> Self {
		Self {
			group: String::new(),
			members: Vec::new(),
			version: String::new(),
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMembership {
	pub group: String,
	pub feed: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PollsMeta {
	pub total: u64,
	pub version: String,
}

impl Default for PollsMeta {
	fn default() -> Self {
		Self {
			total: 0,
			version: String::new(),
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PollVote {
	pub option: String,
	pub votes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PollVotesMeta {
	pub poll: String,
	#[serde(rename = "total_votes")]
	pub total_votes: u64,
	pub version: String,
}

impl Default for PollVotesMeta {
	fn default() -> Self {
		Self {
			poll: String::new(),
			total_votes: 0,
			version: String::new(),
		}
	}
}

#[derive(Debug)]
pub enum RelayIntegrationError {
	Http(reqwest::Error),
	InvalidPostUrl(String),
	MissingPosts(Vec<String>),
	MissingFeedData(String),
	PollParse(String),
}

impl fmt::Display for RelayIntegrationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			RelayIntegrationError::Http(err) => write!(f, "Relay HTTP error: {err}"),
			RelayIntegrationError::InvalidPostUrl(url) => write!(f, "Invalid post URL: {url}"),
			RelayIntegrationError::MissingPosts(posts) => write!(f, "Missing posts: {}", posts.join(", ")),
			RelayIntegrationError::MissingFeedData(feed) => write!(f, "Unable to fetch feed data for {feed}"),
			RelayIntegrationError::PollParse(reason) => write!(f, "Unable to parse poll: {reason}"),
		}
	}
}

impl std::error::Error for RelayIntegrationError {}

/// Result from building a notification view via relay.
pub struct RelayNotificationsResult {
	pub feed: Feed,
	pub notifications: NotificationFeed,
	pub meta: Option<MentionsMeta>,
	pub missing_posts: Vec<String>,
}

/// Result from building a thread view via relay.
pub struct RelayThreadResult {
	pub feed: Feed,
	pub thread: ThreadView,
	pub meta: Option<RepliesMeta>,
	pub missing_posts: Vec<String>,
}

/// Result from fetching poll status via relay.
pub struct RelayPollResult {
	pub poll: Poll,
	pub meta: Option<PollVotesMeta>,
	pub unknown_options: Vec<(String, usize)>,
}

#[derive(Clone)]
struct ResolvedPost {
	post: Post,
	profile: Profile,
}

struct ResolvedPosts {
	posts: Vec<ResolvedPost>,
	missing: Vec<String>,
}

/// Splits a post URL into feed URL and post ID.
/// Expected format: "feed_url#post_id"
fn split_post_url(post_url: &str) -> Result<(String, String), RelayIntegrationError> {
	if let Some((feed, fragment)) = post_url.split_once('#') {
		let feed = feed.trim();
		let fragment = fragment.trim();
		if feed.is_empty() || fragment.is_empty() {
			return Err(RelayIntegrationError::InvalidPostUrl(post_url.to_string()));
		}
		Ok((feed.to_string(), fragment.to_string()))
	} else {
		Err(RelayIntegrationError::InvalidPostUrl(post_url.to_string()))
	}
}

/// Resolves a list of post URLs by fetching their feeds and extracting the posts.
/// Returns resolved posts along with a list of missing post URLs.
async fn resolve_posts_for_urls(
	post_urls: &[String],
	fetch_options: &FetchOptions,
) -> Result<ResolvedPosts, RelayIntegrationError> {
	let mut unique_urls = Vec::new();
	let mut seen: HashSet<String> = HashSet::new();
	for url in post_urls {
		if seen.insert(url.clone()) {
			unique_urls.push(url.clone());
		}
	}

	if unique_urls.is_empty() {
		return Ok(ResolvedPosts { posts: Vec::new(), missing: Vec::new() });
	}

	let mut feed_requests: HashMap<String, Vec<(String, String)>> = HashMap::new();
	for url in unique_urls {
		let (feed_url, post_id) = split_post_url(&url)?;
		feed_requests.entry(feed_url).or_default().push((post_id, url));
	}

	let fetch_targets: Vec<(String, String)> = feed_requests
		.keys()
		.map(|feed| (feed.clone(), feed.clone()))
		.collect();

	let fetched = network::get_feeds_with_options(fetch_targets, fetch_options.clone()).await;
	let mut resolved_posts = Vec::new();
	let mut missing_posts = Vec::new();

	for (profile, posts, source_url) in fetched {
		if let Some(requested_posts) = feed_requests.remove(&source_url) {
			let mut posts_by_id: HashMap<String, Post> = posts
				.into_iter()
				.map(|mut post| {
					if post.author().is_none() {
						post.set_author(profile.nick().to_string());
					}
					(post.id().to_string(), post)
				})
				.collect();

			for (post_id, original_url) in requested_posts {
				if let Some(post) = posts_by_id.remove(&post_id) {
					resolved_posts.push(ResolvedPost {
						post,
						profile: profile.clone(),
					});
				} else {
					missing_posts.push(original_url);
				}
			}
		}
	}

	if !feed_requests.is_empty() {
		let mut remainder = Vec::new();
		for (_feed, requests) in feed_requests.into_iter() {
			remainder.extend(requests.into_iter().map(|(_, url)| url));
		}
		missing_posts.extend(remainder);
	}

	Ok(ResolvedPosts {
		posts: resolved_posts,
		missing: missing_posts,
	})
}

/// Recursively collects all post URLs from a reply tree.
fn collect_reply_urls(nodes: &[ReplyNode], acc: &mut Vec<String>) {
	for node in nodes {
		acc.push(node.post.clone());
		if !node.children.is_empty() {
			collect_reply_urls(&node.children, acc);
		}
	}
}

fn profile_identifier(profile: &Profile) -> String {
	if !profile.nick().is_empty() {
		profile.nick().to_string()
	} else if let Some(source) = profile.source() {
		source.clone()
	} else {
		"unknown".to_string()
	}
}

/// Client for communicating with an Org Social Relay server.
pub struct RelayClient {
	pub base_url: String,
	pub client: Client,
}

impl RelayClient {
	pub fn new(base_url: &str) -> Self {
		RelayClient {
			base_url: base_url.to_string(),
			client: Client::new(),
		}
	}

	fn trimmed_base(&self) -> &str {
		self.base_url.trim_end_matches('/')
	}

	pub async fn get_root_info(&self) -> Result<ApiResponse<RootInfo>, reqwest::Error> {
		let url = format!("{}/", self.trimmed_base());
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<RootInfo>>().await?;
		Ok(resp)
	}

	pub async fn list_feeds(&self) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/feeds/", self.trimmed_base());
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<Vec<String>>>().await?;
		Ok(resp)
	}

	pub async fn add_feed(&self, feed_url: &str) -> Result<ApiResponse<serde_json::Value>, reqwest::Error> {
		let url = format!("{}/feeds/", self.trimmed_base());
		let body = serde_json::json!({"feed": feed_url});
		let resp = self.client.post(&url)
			.json(&body)
			.send()
			.await?
			.json::<ApiResponse<serde_json::Value>>()
			.await?;
		Ok(resp)
	}

	pub async fn get_mentions(&self, feed_url: &str) -> Result<ApiResponse<Vec<String>, MentionsMeta>, reqwest::Error> {
		let encoded_feed = encode(feed_url);
		let url = format!("{}/mentions/?feed={}", self.trimmed_base(), encoded_feed);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>, MentionsMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn get_replies(&self, post_url: &str) -> Result<ApiResponse<Vec<ReplyNode>, RepliesMeta>, reqwest::Error> {
		let encoded_post = encode(post_url);
		let url = format!("{}/replies/?post={}", self.trimmed_base(), encoded_post);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<ReplyNode>, RepliesMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn search(&self, query: &str, page: Option<u32>, per_page: Option<u32>) -> Result<ApiResponse<Vec<String>, SearchMeta>, reqwest::Error> {
		let mut url = format!("{}/search/?q={}", self.trimmed_base(), encode(query));
		if let Some(page) = page {
			url.push_str(&format!("&page={}", page));
		}
		if let Some(per_page) = per_page {
			url.push_str(&format!("&perPage={}", per_page));
		}
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>, SearchMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn search_by_tag(&self, tag: &str, page: Option<u32>, per_page: Option<u32>) -> Result<ApiResponse<Vec<String>, SearchMeta>, reqwest::Error> {
		let mut url = format!("{}/search/?tag={}", self.trimmed_base(), encode(tag));
		if let Some(page) = page {
			url.push_str(&format!("&page={}", page));
		}
		if let Some(per_page) = per_page {
			url.push_str(&format!("&perPage={}", per_page));
		}
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>, SearchMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn list_groups(&self) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/groups/", self.trimmed_base());
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>>>()
			.await?;
		Ok(resp)
	}

	pub async fn register_group_member(&self, group_name: &str, feed_url: &str) -> Result<ApiResponse<GroupMembership>, reqwest::Error> {
		let encoded_group = encode(group_name);
		let encoded_feed = encode(feed_url);
		let url = format!("{}/groups/{}/members/?feed={}", self.trimmed_base(), encoded_group, encoded_feed);
		let resp = self.client.post(&url)
			.send()
			.await?
			.json::<ApiResponse<GroupMembership>>()
			.await?;
		Ok(resp)
	}

	pub async fn get_group_messages(&self, group_name: &str) -> Result<ApiResponse<Vec<GroupMessage>, GroupMessagesMeta>, reqwest::Error> {
		let encoded_group = encode(group_name);
		let url = format!("{}/groups/{}/", self.trimmed_base(), encoded_group);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<GroupMessage>, GroupMessagesMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn list_polls(&self) -> Result<ApiResponse<Vec<String>, PollsMeta>, reqwest::Error> {
		let url = format!("{}/polls/", self.trimmed_base());
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>, PollsMeta>>()
			.await?;
		Ok(resp)
	}

	pub async fn get_poll_votes(&self, post_url: &str) -> Result<ApiResponse<Vec<PollVote>, PollVotesMeta>, reqwest::Error> {
		let encoded_post = encode(post_url);
		let url = format!("{}/polls/votes/?post={}", self.trimmed_base(), encoded_post);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<PollVote>, PollVotesMeta>>()
			.await?;
		Ok(resp)
	}

	/// Builds a notification view by fetching mentions from the relay and resolving them into a feed.
	/// Returns a feed with all mentioned posts, a notification feed view, and any missing post URLs.
	pub async fn build_notification_view(
		&self,
		feed_url: &str,
		user_profile: &Profile,
		user_posts: Vec<Post>,
		fetch_options: Option<FetchOptions>,
	) -> Result<RelayNotificationsResult, RelayIntegrationError> {
		let mentions = self
			.get_mentions(feed_url)
			.await
			.map_err(RelayIntegrationError::Http)?;
		let fetch_options = fetch_options.unwrap_or_default();
		let resolved = resolve_posts_for_urls(&mentions.data, &fetch_options).await?;

		let mut feed = Feed::from_user_posts(user_profile, user_posts);
		let mut existing_profiles: HashSet<String> = feed
			.profiles
			.iter()
			.map(|profile| profile_identifier(profile))
			.collect();

		let mut posts_by_profile: HashMap<String, (Profile, Vec<Post>)> = HashMap::new();
		for resolved_post in resolved.posts.into_iter() {
			let mut post = resolved_post.post;
			let key = profile_identifier(&resolved_post.profile);
			let author = resolved_post.profile.nick().to_string();
			if post.author().is_none() {
				post.set_author(author.clone());
			}
			let entry = posts_by_profile
				.entry(key)
				.or_insert_with(|| (resolved_post.profile.clone(), Vec::new()));
			entry.1.push(post);
		}
		for (profile_key, (profile, posts)) in posts_by_profile.into_iter() {
			if !existing_profiles.contains(&profile_key) {
				feed.add_profile(profile.clone());
				existing_profiles.insert(profile_key.clone());
			}
			let post_ids: Vec<String> = posts.iter().map(|p| p.id().to_string()).collect();
			feed.add_posts(posts);
			feed.set_posts_author(&post_ids, profile.nick());
		}

		let notifications = NotificationFeed::from_feed(&feed, user_profile);
		Ok(RelayNotificationsResult {
			feed,
			notifications,
			meta: mentions.meta,
			missing_posts: resolved.missing,
		})
	}

	/// Builds a thread view by fetching replies from the relay and resolving them into a feed.
	/// Returns a feed with all thread posts, a thread view, and any missing post URLs.
	pub async fn build_thread_view(
		&self,
		post_url: &str,
		fetch_options: Option<FetchOptions>,
	) -> Result<RelayThreadResult, RelayIntegrationError> {
		let replies = self
			.get_replies(post_url)
			.await
			.map_err(RelayIntegrationError::Http)?;

		let fetch_options = fetch_options.unwrap_or_default();
		let mut urls = Vec::new();
		collect_reply_urls(&replies.data, &mut urls);
		urls.push(post_url.to_string());
		let resolved = resolve_posts_for_urls(&urls, &fetch_options).await?;

		let mut posts: Vec<Post> = Vec::new();
		let mut profiles: HashMap<String, Profile> = HashMap::new();
		for resolved_post in resolved.posts.into_iter() {
			let mut post = resolved_post.post;
			let key = profile_identifier(&resolved_post.profile);
			if post.author().is_none() {
				post.set_author(resolved_post.profile.nick().to_string());
			}
			profiles.entry(key).or_insert_with(|| resolved_post.profile.clone());
			posts.push(post);
		}

		if posts.is_empty() {
			return Err(RelayIntegrationError::MissingPosts(vec![post_url.to_string()]));
		}

		let feed = Feed::from_posts_and_profiles(posts, profiles.into_values().collect());
		let thread = ThreadView::from_feed(&feed);
		Ok(RelayThreadResult {
			feed,
			thread,
			meta: replies.meta,
			missing_posts: resolved.missing,
		})
	}

	/// Fetches poll status from the relay and populates vote counts.
	/// Returns a poll with updated vote counts and any unknown options.
	pub async fn fetch_poll_status(
		&self,
		poll_post_url: &str,
		poll_post: &Post,
	) -> Result<RelayPollResult, RelayIntegrationError> {
		let mut poll = poll::parse_poll_from_post(poll_post)
			.ok_or_else(|| RelayIntegrationError::PollParse("Post does not contain a valid poll".to_string()))?;
		poll.clear_votes();

		let response = self
			.get_poll_votes(poll_post_url)
			.await
			.map_err(RelayIntegrationError::Http)?;

		let mut unknown_options = Vec::new();
		for vote_group in response.data.iter() {
			let option_label = vote_group.option.trim();
			let has_option = poll
				.options
				.iter()
				.any(|opt| opt.text.trim().eq_ignore_ascii_case(option_label));
			if has_option {
				for _ in 0..vote_group.votes.len() {
					poll.add_vote_by_text(option_label);
				}
			} else {
				unknown_options.push((vote_group.option.clone(), vote_group.votes.len()));
			}
		}

		poll.update_status();
		Ok(RelayPollResult {
			poll,
			meta: response.meta,
			unknown_options,
		})
	}
}
