use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionTool, CreateChatCompletionRequestArgs,
        CreateChatCompletionResponse, CreateCompletionRequestArgs, CreateCompletionResponse,
        Prompt,
    },
};

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
        Self { client }
    }

    pub async fn completion<V: Into<Prompt>>(
        &self,
        model: &str,
        prompt: V,
    ) -> Result<CreateCompletionResponse, OpenAIError> {
        // Create a `CreateCompletionRequest`
        let request = CreateCompletionRequestArgs::default()
            .model(model)
            .prompt(prompt)
            .max_tokens(100_u32)
            .build()?;

        let response = self.client.completions().create(request).await?;
        Ok(response)
    }

    pub async fn chat<
        M: Into<Vec<ChatCompletionRequestMessage>>,
        T: Into<Vec<ChatCompletionTool>>,
    >(
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

        let response = self.client.chat().create(request).await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    async fn openai_local_llama32_demo() {
        let api_key = ""; //env!("OPENAI_API_KEY");
        let api_base = "http://localhost:11434/v1";

        let client = LlmClient::new(api_key, api_base);

        // let model = "gpt-3.5-turbo-instruct";
        let model = "llama3.2";
        let prompt = "Tell me the recipe of alfredo pasta";
        let response = client
            .completion(model, prompt)
            .await
            .expect("Failed to request model completion");

        println!("{}", response.choices.first().unwrap().text);
    }

    #[tokio::test]
    async fn basic_planner() {
        use crate::{
            ConversationHistory, Function,
            plan::{BasicPlanner, PlanningLoop},
            tools::ReadEmailsArgs,
        };
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
            ChatCompletionToolArgs, FunctionObject,
        };
        use serde_json::json;
        let system_message = "You are a helpful email assistant with the ability to summarize emails.
            You have access to the following Rust tools:
            1. `read_emails(count: usize) -> Vec<HashMap>`: Reads the top n emails from the user's mailbox.

            The user's Team alias is: bob.sheffield@contoso.com";
        let tools = vec![
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_emails".to_string(),
                    description: Some(
                        "Reading a number of {count} email from the inbox".to_string(),
                    ),
                    parameters: Some(json!({ "count": "usize" })),
                    strict: Some(true),
                })
                .build()
                .unwrap(),
        ];

        let basic_planner = BasicPlanner::new(tools.clone());

        let api_key = ""; //env!("OPENAI_API_KEY");
        let api_base = "http://localhost:11434/v1";

        let client = LlmClient::new(api_key, api_base);

        // Build a system message
        let system_request = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_message)
            .build()
            .unwrap()
            .into();
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content("Write me a summary of my 5 most recent emails.")
            .build()
            .unwrap()
            .into();

        let state: crate::State = ConversationHistory(vec![system_request, user_message]);
        let chat_request = client.chat(state.0.clone(), tools);
        let current_message = chat_request.await.unwrap().choices[0].message.clone();

        let mut planning_loop = PlanningLoop::new(
            basic_planner,
            client,
            vec![Function("read_emails".to_string())],
        );

        let mut datastore = crate::Datastore;
        planning_loop
            .run(state, &mut datastore, crate::Message::Chat(current_message))
            .await
            .expect("Failed to run");
    }
}
