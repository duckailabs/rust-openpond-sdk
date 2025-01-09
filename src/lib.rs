mod error;
mod types;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

pub use error::{OpenPondError, Result};
pub use types::*;

/// OpenPond SDK for interacting with the P2P network.
///
/// The SDK can be used in two ways:
/// 1. With a private key - Creates your own agent identity with full control
/// 2. Without a private key - Uses a hosted agent
///
/// Both modes can optionally use an apiKey for authenticated access.
#[derive(Clone)]
pub struct OpenPondSDK {
    client: reqwest::Client,
    config: OpenPondConfig,
    last_message_timestamp: Arc<Mutex<i64>>,
    message_callback: Arc<Mutex<Option<Box<dyn Fn(Message) + Send + Sync>>>>,
    error_callback: Arc<Mutex<Option<Box<dyn Fn(OpenPondError) + Send + Sync>>>>,
}

impl OpenPondSDK {
    /// Creates a new instance of the OpenPond SDK
    pub fn new(config: OpenPondConfig) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        if let Some(api_key) = &config.api_key {
            headers.insert(
                "X-API-Key",
                api_key.parse().unwrap(),
            );
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            config,
            last_message_timestamp: Arc::new(Mutex::new(0)),
            message_callback: Arc::new(Mutex::new(None)),
            error_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Set callback for receiving messages
    pub async fn on_message<F>(&self, callback: F)
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let mut cb = self.message_callback.lock().await;
        *cb = Some(Box::new(callback));
    }

    /// Set callback for handling errors
    pub async fn on_error<F>(&self, callback: F)
    where
        F: Fn(OpenPondError) + Send + Sync + 'static,
    {
        let mut cb = self.error_callback.lock().await;
        *cb = Some(Box::new(callback));
    }

    /// Starts the SDK and begins listening for messages
    pub async fn start(&self) -> Result<()> {
        // Register the agent if not already registered
        self.register_agent().await?;

        // Start polling for new messages
        self.start_polling();
        Ok(())
    }

    /// Sends a message to another agent
    pub async fn send_message(
        &self,
        to_agent_id: &str,
        content: &str,
        options: Option<SendMessageOptions>,
    ) -> Result<String> {
        let response = self.client
            .post(&format!("{}/messages", self.config.api_url))
            .json(&serde_json::json!({
                "toAgentId": to_agent_id,
                "content": content,
                "privateKey": self.config.private_key,
                "options": options,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OpenPondError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data["messageId"].as_str().unwrap_or_default().to_string())
    }

    /// Gets information about an agent
    pub async fn get_agent(&self, agent_id: &str) -> Result<Agent> {
        let response = self.client
            .get(&format!("{}/agents/{}", self.config.api_url, agent_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OpenPondError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        Ok(response.json().await?)
    }

    /// Lists all registered agents
    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let response = self.client
            .get(&format!("{}/agents", self.config.api_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(OpenPondError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        let data: serde_json::Value = response.json().await?;
        Ok(serde_json::from_value(data["agents"].clone())?)
    }

    async fn register_agent(&self) -> Result<()> {
        let response = self.client
            .post(&format!("{}/agents/register", self.config.api_url))
            .json(&serde_json::json!({
                "privateKey": self.config.private_key,
                "name": self.config.agent_name,
            }))
            .send()
            .await?;

        // Ignore 409 Conflict (already registered)
        if !response.status().is_success() && response.status() != reqwest::StatusCode::CONFLICT {
            return Err(OpenPondError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        Ok(())
    }

    fn start_polling(&self) {
        let client = self.client.clone();
        let last_timestamp = Arc::clone(&self.last_message_timestamp);
        let message_callback = Arc::clone(&self.message_callback);
        let error_callback = Arc::clone(&self.error_callback);
        let api_url = self.config.api_url.clone();
        let private_key = self.config.private_key.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;
                let since = {
                    let timestamp = last_timestamp.lock().await;
                    *timestamp
                };

                let response = client
                    .get(&format!("{}/messages", api_url))
                    .query(&[
                        ("privateKey", private_key.as_deref().unwrap_or("")),
                        ("since", &since.to_string()),
                    ])
                    .send()
                    .await;

                match response {
                    Ok(response) => {
                        if !response.status().is_success() {
                            if let Some(cb) = error_callback.lock().await.as_ref() {
                                cb(OpenPondError::ApiError {
                                    status: response.status().as_u16(),
                                    message: response.text().await.unwrap_or_default(),
                                });
                            }
                            continue;
                        }

                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<serde_json::Value>(&text) {
                                    Ok(data) => {
                                        if let Ok(messages) = serde_json::from_value::<Vec<Message>>(data["messages"].clone()) {
                                            let mut max_timestamp = since;
                                            if let Some(cb) = message_callback.lock().await.as_ref() {
                                                for message in messages {
                                                    max_timestamp = std::cmp::max(max_timestamp, message.timestamp);
                                                    cb(message);
                                                }
                                            }
                                            *last_timestamp.lock().await = max_timestamp;
                                        }
                                    }
                                    Err(e) => {
                                        if let Some(cb) = error_callback.lock().await.as_ref() {
                                            cb(OpenPondError::SerializationError(e));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some(cb) = error_callback.lock().await.as_ref() {
                                    cb(OpenPondError::NetworkError(e.to_string()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(cb) = error_callback.lock().await.as_ref() {
                            cb(OpenPondError::NetworkError(e.to_string()));
                        }
                    }
                }
            }
        });
    }
}
