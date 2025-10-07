//! Relay API client
//! Implements the spec of the Org Social Relay API
//! This module is yet to be integrated with other functionality

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use urlencoding::encode;

/// Common API response wrapper
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

/// Replies tree response
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

/// Search response
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

/// Group message tree
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

/// Client for communicating with Org Social Relay API
pub struct RelayClient {
	pub base_url: String,
	pub client: Client,
}

impl RelayClient {
	/// Create a new RelayClient
	pub fn new(base_url: &str) -> Self {
		RelayClient {
			base_url: base_url.to_string(),
			client: Client::new(),
		}
	}

	fn trimmed_base(&self) -> &str {
		self.base_url.trim_end_matches('/')
	}

	/// Get root info from relay
	pub async fn get_root_info(&self) -> Result<ApiResponse<RootInfo>, reqwest::Error> {
		let url = format!("{}/", self.trimmed_base());
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<RootInfo>>().await?;
		Ok(resp)
	}

	/// List all registered feeds
	pub async fn list_feeds(&self) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/feeds/", self.trimmed_base());
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<Vec<String>>>().await?;
		Ok(resp)
	}

	/// Add a new feed
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

	/// Get mentions for a feed
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

	/// Get replies for a post
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

	/// Search posts by query or tag
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

	/// Search posts by tag
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

	/// List all groups
	pub async fn list_groups(&self) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/groups/", self.trimmed_base());
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>>>()
			.await?;
		Ok(resp)
	}

	/// Register a feed as a group member
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

	/// Get messages from a group
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

	/// List all polls
	pub async fn list_polls(&self) -> Result<ApiResponse<Vec<String>, PollsMeta>, reqwest::Error> {
		let url = format!("{}/polls/", self.trimmed_base());
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>, PollsMeta>>()
			.await?;
		Ok(resp)
	}

	/// Get vote breakdown for a poll
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
}
