//! Relay API client
//! Implements the initial, unreleased specification of the Org Social Relay API
//! This module is yet to be integrated with other functionality

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use reqwest::Client;

/// Common API response wrapper
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T> {
	pub r#type: String,
	pub errors: Vec<String>,
	pub data: T,
	#[serde(default)]
	pub meta: Option<serde_json::Value>,
}

/// Root endpoint link info
#[derive(Debug, Deserialize, Serialize)]
pub struct RootLink {
	pub rel: String,
	pub href: String,
	pub method: String,
}

/// Feeds list response
#[derive(Debug, Deserialize, Serialize)]
pub struct FeedsList {
	pub feeds: Vec<String>,
}

/// Mentions response
#[derive(Debug, Deserialize, Serialize)]
pub struct Mentions {
	pub mentions: Vec<String>,
	pub meta: Option<MentionsMeta>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MentionsMeta {
	pub feed: String,
	pub total: u32,
	pub version: String,
}

/// Replies tree response
#[derive(Debug, Deserialize, Serialize)]
pub struct ReplyNode {
	pub post: String,
	pub children: Vec<ReplyNode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepliesMeta {
	pub parent: String,
	pub version: String,
}

/// Search response
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchMeta {
	pub version: String,
	pub query: String,
	pub total: u32,
	pub page: u32,
	#[serde(rename = "perPage")]
	pub per_page: u32,
	#[serde(rename = "hasNext")]
	pub has_next: bool,
	#[serde(rename = "hasPrevious")]
	pub has_previous: bool,
	pub links: Option<HashMap<String, Option<String>>>,
}

/// Group info
#[derive(Debug, Deserialize, Serialize)]
pub struct Group {
	pub id: u32,
	pub name: String,
	pub description: String,
	pub members: u32,
	pub posts: u32,
}

/// Group message tree
#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMessage {
	pub post: String,
	pub children: Vec<GroupMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupMessagesMeta {
	pub group: String,
	pub total: u32,
	pub version: String,
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

	/// Get root info from relay
	pub async fn get_root_info(&self) -> Result<ApiResponse<Vec<RootLink>>, reqwest::Error> {
		let url = format!("{}/", self.base_url.trim_end_matches('/'));
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<Vec<RootLink>>>().await?;
		Ok(resp)
	}

	/// List all registered feeds
	pub async fn list_feeds(&self) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/feeds", self.base_url.trim_end_matches('/'));
		let resp = self.client.get(&url).send().await?.json::<ApiResponse<Vec<String>>>().await?;
		Ok(resp)
	}

	/// Add a new feed
	pub async fn add_feed(&self, feed_url: &str) -> Result<ApiResponse<serde_json::Value>, reqwest::Error> {
		let url = format!("{}/feeds", self.base_url.trim_end_matches('/'));
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
	pub async fn get_mentions(&self, feed_url: &str) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let url = format!("{}/mentions/?feed={}", self.base_url.trim_end_matches('/'), feed_url);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>>>()
			.await?;
		Ok(resp)
	}

	/// Get replies for a post
	pub async fn get_replies(&self, post_url: &str) -> Result<ApiResponse<Vec<ReplyNode>>, reqwest::Error> {
		let url = format!("{}/replies/?post={}", self.base_url.trim_end_matches('/'), post_url);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<ReplyNode>>>()
			.await?;
		Ok(resp)
	}

	/// Search posts by query or tag
	pub async fn search(&self, query: &str, page: Option<u32>, per_page: Option<u32>) -> Result<ApiResponse<Vec<String>>, reqwest::Error> {
		let mut url = format!("{}/search?q={}", self.base_url.trim_end_matches('/'), query);
		if let Some(page) = page {
			url.push_str(&format!("&page={}", page));
		}
		if let Some(per_page) = per_page {
			url.push_str(&format!("&perPage={}", per_page));
		}
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<String>>>()
			.await?;
		Ok(resp)
	}

	/// List all groups
	pub async fn list_groups(&self) -> Result<ApiResponse<Vec<Group>>, reqwest::Error> {
		let url = format!("{}/groups", self.base_url.trim_end_matches('/'));
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<Group>>>()
			.await?;
		Ok(resp)
	}

	/// Register a feed as a group member
	pub async fn register_group_member(&self, group_id: u32, feed_url: &str) -> Result<ApiResponse<serde_json::Value>, reqwest::Error> {
		let url = format!("{}/groups/{}/members?feed={}", self.base_url.trim_end_matches('/'), group_id, feed_url);
		let resp = self.client.post(&url)
			.send()
			.await?
			.json::<ApiResponse<serde_json::Value>>()
			.await?;
		Ok(resp)
	}

	/// Get messages from a group
	pub async fn get_group_messages(&self, group_id: u32) -> Result<ApiResponse<Vec<GroupMessage>>, reqwest::Error> {
		let url = format!("{}/groups/{}/messages", self.base_url.trim_end_matches('/'), group_id);
		let resp = self.client.get(&url)
			.send()
			.await?
			.json::<ApiResponse<Vec<GroupMessage>>>()
			.await?;
		Ok(resp)
	}
}
