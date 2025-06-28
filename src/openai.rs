use async_openai::{Client, config::OpenAIConfig, types::{CreateCompletionRequestArgs, CreateChatCompletionRequestArgs, CreateCompletionResponse, Prompt, ChatCompletionRequestMessage, ChatCompletionTool, CreateChatCompletionResponse}, error::OpenAIError};

pub struct LlmClient {
    client: Client<OpenAIConfig>,
}

impl LlmClient {
    pub fn new(api_key: &str, api_base: &str) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base)
            .with_org_id("buciumede");

        let client = Client::with_config(config);
        Self {
            client
        }
    }

    pub async fn completion<V: Into<Prompt>>(&self, model: &str, prompt: V) -> Result<CreateCompletionResponse, OpenAIError> {
        // Create a `CreateCompletionRequest`
        let request = CreateCompletionRequestArgs::default()
            .model(model)
            .prompt(prompt)
            .max_tokens(100_u32)
            .build()?;

        let response = self.client
            .completions()
            .create(request)
            .await?;
        Ok(response)
    }

    pub async fn chat<M: Into<Vec<ChatCompletionRequestMessage>>, T: Into<Vec<ChatCompletionTool>>>(
        &self,
        messages: M,
        tools: T,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        let model = "llama3.2";
        // Create a `CreateCompletionRequest`
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .tools(tools)
            .parallel_tool_calls(false)
            .max_completion_tokens(500_u32)
            .build()?;

        let response = self.client
            .chat()
            .create(request)
            .await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_local_llama32_demo() {
        let api_key = ""; //env!("OPENAI_API_KEY");
        let api_base = "http://localhost:11434/v1";

        let client = LlmClient::new(api_key, api_base);

        // let model = "gpt-3.5-turbo-instruct";
        let model = "llama3.2";
        let prompt = "Tell me the recipe of alfredo pasta";
        let response = client.completion(model, prompt).await.expect("Failed to request model completion");

        println!("{}", response.choices.first().unwrap().text);
    }
}
