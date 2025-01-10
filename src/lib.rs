mod error;
mod types;

use std::sync::Arc;
use tokio::sync::Mutex;
use eventsource_client::{Client as EventSourceClient, SSE};

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
    message_callback: Arc<Mutex<Option<Box<dyn Fn(Message) + Send + Sync>>>>,
    error_callback: Arc<Mutex<Option<Box<dyn Fn(OpenPondError) + Send + Sync>>>>,
    sse_client: Arc<Mutex<Option<EventSourceClient>>>,
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
            message_callback: Arc::new(Mutex::new(None)),
            error_callback: Arc::new(Mutex::new(None)),
            sse_client: Arc::new(Mutex::new(None)),
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

    /// Starts the SDK and begins listening for messages using SSE
    pub async fn start(&self) -> Result<()> {
        // Register the agent if not already registered
        self.register_agent().await?;

        // Setup SSE client
        let url = format!("{}/messages/stream", self.config.api_url);
        let mut client = EventSourceClient::new(url)?;

        // Store the client for later use (e.g., cleanup)
        {
            let mut sse = self.sse_client.lock().await;
            *sse = Some(client.clone());
        }

        // Clone the callbacks for the async task
        let message_callback = self.message_callback.clone();
        let error_callback = self.error_callback.clone();

        // Start listening for events in a separate task
        tokio::spawn(async move {
            while let Ok(event) = client.next().await {
                match event {
                    SSE::Event(event) => {
                        if let Ok(msg) = serde_json::from_str::<Message>(&event.data) {
                            if let Some(cb) = message_callback.lock().await.as_ref() {
                                cb(msg);
                            }
                        }
                    }
                    SSE::Error(error) => {
                        if let Some(cb) = error_callback.lock().await.as_ref() {
                            cb(OpenPondError::NetworkError(error.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Stops the SDK and cleans up resources
    pub async fn stop(&self) -> Result<()> {
        let mut sse = self.sse_client.lock().await;
        if let Some(client) = sse.take() {
            client.close();
        }
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
}
