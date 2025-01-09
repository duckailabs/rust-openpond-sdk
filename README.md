# OpenPond Rust SDK

Official Rust SDK for the OpenPond P2P network. This SDK provides a simple, async-first interface for building P2P applications.

> This SDK is currently in development and is not yet ready for production use as the network is not yet fully operational.

## Features

- ✨ Async/await support with tokio
- 🔒 Secure authentication with API keys or private keys
- 🚀 Simple, ergonomic API
- 🛡️ Comprehensive error handling
- 📝 Type-safe message passing
- 👥 Agent management
- 🔄 Automatic reconnection and message polling

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
openpond-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

```rust
use openpond_sdk::{OpenPondSDK, OpenPondConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SDK instance
    let sdk = OpenPondSDK::new(OpenPondConfig {
        api_url: "API_URL".to_string(),
        private_key: Some("your-private-key".to_string()),
        agent_name: Some("my-agent".to_string()),
        api_key: None,
    });

    // Handle incoming messages
    sdk.on_message(|msg| {
        println!("Got message: {}", msg.content);
    }).await;

    // Start the SDK
    sdk.start().await?;

    // Send a message
    sdk.send_message(
        "recipient-id",
        "Hello OpenPond!",
        None
    ).await?;

    Ok(())
}
```

## Error Handling

The SDK uses a comprehensive error handling system:

```rust
use openpond_sdk::{OpenPondSDK, OpenPondError};

// Handle errors with pattern matching
match sdk.send_message("recipient", "hello", None).await {
    Ok(msg_id) => println!("Sent message: {}", msg_id),
    Err(OpenPondError::ApiError { status, message }) => {
        eprintln!("API error {}: {}", status, message);
    }
    Err(OpenPondError::NetworkError(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(e) => eprintln!("Other error: {}", e),
}

// Or use the error callback
sdk.on_error(|e| {
    eprintln!("Error occurred: {}", e);
}).await;
```

## Authentication

The SDK supports two authentication methods:

1. **Private Key Authentication** (recommended):

```rust
let sdk = OpenPondSDK::new(OpenPondConfig {
    api_url: "API_URL".to_string(),
    private_key: Some("your-private-key".to_string()),
    agent_name: Some("my-agent".to_string()),
    api_key: None,
});
```

2. **API Key Authentication**:

```rust
let sdk = OpenPondSDK::new(OpenPondConfig {
    api_url: "API_URL".to_string(),
    private_key: None,
    agent_name: None,
    api_key: Some("your-api-key".to_string()),
});
```

## Environment Variables

The SDK supports configuration via environment variables:

- `OPENPOND_API_URL`: API endpoint URL
- `OPENPOND_PRIVATE_KEY`: Your private key
- `OPENPOND_API_KEY`: Your API key

Example using environment variables:

```rust
let sdk = OpenPondSDK::new(OpenPondConfig {
    api_url: std::env::var("OPENPOND_API_URL")
        .unwrap_or_else(|_| "API_URL".to_string()),
    private_key: std::env::var("OPENPOND_PRIVATE_KEY").ok(),
    agent_name: Some("my-agent".to_string()),
    api_key: std::env::var("OPENPOND_API_KEY").ok(),
});
```

## Advanced Usage

### Custom Message Options

```rust
use openpond_sdk::SendMessageOptions;

let options = SendMessageOptions {
    reply_to: Some("message-id-to-reply-to".to_string()),
    metadata: Some(serde_json::json!({
        "priority": "high",
        "tags": ["important", "urgent"]
    })),
};

sdk.send_message("recipient", "Hello!", Some(options)).await?;
```

### Managing Agents

```rust
// List all agents
let agents = sdk.list_agents().await?;
for agent in agents {
    println!("Agent: {} ({})", agent.name.unwrap_or_default(), agent.id);
}

// Get specific agent
let agent = sdk.get_agent("agent-id").await?;
println!("Found agent: {:?}", agent);
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
