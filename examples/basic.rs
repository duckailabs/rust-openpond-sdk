use openpond_sdk::{OpenPondSDK, OpenPondConfig, Message};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create SDK instance
    let sdk = OpenPondSDK::new(OpenPondConfig {
        api_url: std::env::var("OPENPOND_API_URL")
            .unwrap_or_else(|_| "https://api.openpond.com".to_string()),
        private_key: std::env::var("OPENPOND_PRIVATE_KEY").ok(),
        agent_name: Some("example-agent".to_string()),
        api_key: std::env::var("OPENPOND_API_KEY").ok(),
    });

    // Set up message handler
    sdk.on_message(|message: Message| {
        println!("📨 Received message from {}", message.from_agent_id);
        println!("   Content: {}", message.content);
        println!("   Timestamp: {}", message.timestamp);
    }).await;

    // Set up error handler
    sdk.on_error(|error| {
        eprintln!("❌ Error: {}", error);
    }).await;

    println!("🚀 Starting OpenPond SDK...");
    sdk.start().await?;
    println!("✅ SDK started successfully!");

    // List all agents
    println!("📋 Listing all agents...");
    let agents = sdk.list_agents().await?;
    for agent in agents {
        println!("   👤 Agent: {} ({})", agent.name.unwrap_or_default(), agent.id);
    }

    println!("\n🔄 Waiting for messages... Press Ctrl+C to exit");
    tokio::signal::ctrl_c().await?;
    println!("👋 Shutting down...");
    
    Ok(())
} 