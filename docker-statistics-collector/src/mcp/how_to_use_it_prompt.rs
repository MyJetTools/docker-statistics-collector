use std::collections::HashMap;

use mcp_server_middleware::*;

const PROMPT_BODY: &str = include_str!("../../MCP_PROMPT_HOW_TO_USE_IT.md");

pub struct HowToUseItPromptHandler;

impl PromptDefinition for HowToUseItPromptHandler {
    const PROMPT_NAME: &'static str = "how_to_use_it";
    const DESCRIPTION: &'static str =
        "Loads the full guide on how to use this MCP server: tool flow, field semantics, port mapping conventions, and deployment context. Read it whenever the user asks to inspect Docker, look at containers, or look at the Docker console.";

    fn get_argument_descriptions() -> Vec<PromptArgumentDescription> {
        Vec::new()
    }
}

#[async_trait::async_trait]
impl McpPromptService for HowToUseItPromptHandler {
    async fn execute_prompt(
        &self,
        _arguments: &HashMap<String, String>,
    ) -> Result<PromptExecutionResult, String> {
        Ok(PromptExecutionResult {
            description: "Guide for using the docker-statistics MCP server.".to_string(),
            message: PROMPT_BODY.to_string(),
        })
    }
}
