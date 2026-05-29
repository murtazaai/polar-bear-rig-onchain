//! Live provider tests - gated behind `#[ignore]`.
//!
//! These tests make real calls to the Anthropic API and require a valid
//! `ANTHROPIC_API_KEY` in the environment. They are skipped during CI (no key
//! available) and must be run explicitly:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-... cargo test --test providers -- --ignored --test-threads=1
//! ```

use polar_bear_rig_onchain::config::Config;

/// The rig-core Anthropic client must construct successfully when
/// `ANTHROPIC_API_KEY` is set.
///
/// Uses `#[ignore]` so this test does not block CI pipelines that lack an
/// API key.
#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY - run with --ignored"]
async fn test_anthropic_client_builds_with_valid_key() {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env().expect("config must load");
    let api_key = cfg
        .anthropic_api_key
        .as_deref()
        .expect("ANTHROPIC_API_KEY must be set for this test");
    let result = rig_core::providers::anthropic::Client::new(api_key);
    assert!(
        result.is_ok(),
        "Client::new must succeed with a valid API key"
    );
}

/// A minimal rig-core agent prompt must return a non-empty response.
#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY - run with --ignored"]
async fn test_agent_returns_non_empty_response() {
    use rig_core::{
        client::{CompletionClient, ProviderClient},
        completion::Prompt,
        providers::anthropic,
    };

    dotenvy::dotenv().ok();
    let cfg = Config::from_env().expect("config must load");
    let api_key = cfg
        .anthropic_api_key
        .as_deref()
        .expect("ANTHROPIC_API_KEY must be set for this test");

    let client = anthropic::Client::new(api_key).unwrap();
    let agent = client
        .agent("claude-haiku-4-5-20251001")
        .preamble("Reply with exactly the word: PONG")
        .build();

    let response = agent.prompt("PING").await.unwrap();
    assert!(!response.is_empty(), "agent response must not be empty");
    assert!(
        response.to_uppercase().contains("PONG"),
        "agent must follow the preamble instruction"
    );
}
