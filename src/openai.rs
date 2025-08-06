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

    pub fn local_llama31() -> Self {
        let api_key = "";
        let api_base = "http://localhost:11434/v1";
        Self::new(api_key, api_base)
    }

    pub fn openai() -> Self {
        let api_key = env!("OPENAI_API_KEY");
        let api_base = "https://api.openai.com/v1";
        Self::new(api_key, api_base)
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
        let model = "gpt-4o-mini";
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
    use crate::tools::variable_schema_gen;

    // #[tokio::test]
    async fn _openai_local_llama32_demo() {
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

    // #[tokio::test]
    async fn _basic_planner() {
        use crate::{
            ConversationHistory, Function,
            plan::{BasicPlanner, PlanningLoop},
        };
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
            ChatCompletionToolArgs, ChatCompletionToolType, FunctionObject,
        };
        use serde_json::json;
        let system_message = "You are a helpful email assistant with the ability to summarize emails and to send Slack messages.
            You have access to the following Rust tools:
            1. `read_emails(count: usize) -> Vec<HashMap>`: Reads the top n emails from the user's mailbox.
            2. `send_slack_message(channel: String, message: String, preview: bool) -> String`: Sends a message to a Slack channel.

            All arguments to tools have an `anyOf` schema, with a `kind` tag indicating whether the value is a literal value (`value`) or a variable name (`variable_name`).
            When choosing tool call arguments, make sure to use the `kind` tag to indicate whether the value is a literal value or a variable name.
            - If `kind` == \"value\", the value MUST be passed in the `value` field.
            - If `kind` == \"variable\", a variable name MUST be passed in the `variable` field instead.
            Make absolutely sure to respect this convention. You MUST NOT pass a variable name in the `value` field or vice versa.

            The user's Slack alias is: bob.sheffield@contoso.com";
        let tools =
            vec![
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_emails".to_string(),
                    description: Some(
                        "Reading a number of {count} email from the inbox".to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "string",
                                "description": "The number of emails to read",
                            },
                        },
                        "required": ["count"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .build()
                .unwrap(),
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "send_slack_message".to_string(),
                    description: Some(
                        "Sends a {message} to a slack {channel} with an optional {preview}"
                            .to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "channel": {
                                "type": "string",
                                "description": "The channel where the message should be sent",
                            },
                            "message": {
                                "type": "string",
                                "description": "The message to be sent",
                            },
                            "preview": {
                                "type": "string",
                                "description": "Whether or not to include the link preview",
                            },
                        },
                        "required": ["channel", "message", "preview"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .r#type(ChatCompletionToolType::Function)
                .build()
                .unwrap(),
        ];

        let basic_planner = BasicPlanner::new(tools.clone());

        let client = LlmClient::openai();

        // Build a system message
        let system_request = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_message)
            .build()
            .unwrap()
            .into();
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content("Write a summary of my 5 most recent emails and send it to me as private Slack message.")
            .build()
            .unwrap()
            .into();

        let state: crate::State = ConversationHistory(vec![system_request, user_message]);
        let chat_request = client.chat(state.0.clone(), tools);
        let current_message = chat_request.await.unwrap().choices[0].message.clone();

        let mut planning_loop = PlanningLoop::new(
            basic_planner,
            client,
            vec![
                Function::new("read_emails".to_string()),
                Function::new("send_slack_message".to_string()),
            ],
        );

        let mut datastore = crate::Datastore;
        let response = planning_loop
            .run(state, &mut datastore, crate::Message::Chat(current_message))
            .await
            .expect("Failed to run");
        println!("{response:#?}");
    }

    //#[tokio::test]
    async fn _var_planner() {
        use crate::{
            ConversationHistory, Function,
            plan::{PlanningLoop, VarPlanner},
        };
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
            ChatCompletionToolArgs, ChatCompletionToolType, FunctionObject,
        };
        use serde_json::json;
        let system_message = "You are a helpful email assistant with the ability to summarize emails and to send Slack messages.
            You have access to the following Rust tools:
            1. `read_emails(count: usize) -> Vec<HashMap>`: Reads the top n emails from the user's mailbox.
            2. `send_slack_message(channel: String, message: String, preview: bool) -> String`: Sends a message to a Slack channel.
            3. `read_variable(variable: String) -> String`: Reads the contents of a variable to obtain results of a tool call.

            Whenever you call a tool, you will not receive the result directly. Rather, a variable standing in for the result will be appended to the conversation. You can use the `read_variable` tool to read the contents of a variable and obtain the results of the previous tool call if you MUST know it before the next tool call.

            All arguments to tools have an `anyOf` schema, with a `kind` tag indicating whether the value is a literal value (`value`) or a variable name (`variable_name`).
            When choosing tool call arguments, make sure to use the `kind` tag to indicate whether the value is a literal value or a variable name.
            - If `kind` == \"value\", the value MUST be passed in the `value` field.
            - If `kind` == \"variable\", a variable name MUST be passed in the `variable` field instead.
            Make absolutely sure to respect this convention. You MUST NOT pass a variable name in the `value` field or vice versa.

            If you are not sure about the contents of data pertaining to the user’s request, use `read_variable` or gather the relevant information from other tools: do NOT guess or make up an answer.
            The user's Slack alias is: bob.sheffield@contoso.com";
        let tools =
            vec![
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_emails".to_string(),
                    description: Some(
                        "Reading a number of {count} email from the inbox".to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "string",
                                "description": "The number of emails to read",
                            },
                        },
                        "required": ["count"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .build()
                .unwrap(),
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "send_slack_message".to_string(),
                    description: Some(
                        "Sends a {message} to a slack {channel} with an optional {preview}"
                            .to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "channel": {
                                "type": "string",
                                "description": "The channel where the message should be sent",
                            },
                            "message": {
                                "type": "string",
                                "description": "The message to be sent",
                            },
                            "preview": {
                                "type": "string",
                                "description": "Whether or not to include the link preview",
                            },
                        },
                        "required": ["channel", "message", "preview"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .r#type(ChatCompletionToolType::Function)
                .build()
                .unwrap(),
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_variable".to_string(),
                    description: Some(
                        "Read a {variable} name that save a tool result to obtain the contents"
                            .to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "variable": {
                                "type": "string",
                                "description": "The variable to be read",
                            },
                        },
                        "required": ["variable"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .r#type(ChatCompletionToolType::Function)
                .build()
                .unwrap(),
        ];

        let var_planner = VarPlanner::new(tools.clone());

        let client = LlmClient::openai();
        //let client = LlmClient::local_llama31();

        // Build a system message
        let system_request = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_message)
            .build()
            .unwrap()
            .into();
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content("Write a summary of my 5 most recent emails and send it to me as private Slack message.")
            .build()
            .unwrap()
            .into();

        let state: crate::State = ConversationHistory(vec![system_request, user_message]);
        let chat_request = client.chat(state.0.clone(), tools);
        let current_message = chat_request.await.unwrap().choices[0].message.clone();

        let mut planning_loop = PlanningLoop::new(
            var_planner,
            client,
            vec![
                Function::new("read_emails".to_string()),
                Function::new("send_slack_message".to_string()),
                Function::new("read_variable".to_string()),
            ],
        );

        let mut datastore = crate::Datastore;
        let response = planning_loop
            .run(state, &mut datastore, crate::Message::Chat(current_message))
            .await
            .expect("Failed to run");
        println!("{response:#?}");
    }

    #[tokio::test]
    async fn taint_tracking_planner() {
        use crate::{
            Integrity, Message, MetaFunction, Policy,
            plan::{PlanningLoop, TaintTrackingPlanner},
        };
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
            ChatCompletionToolArgs, ChatCompletionToolType, FunctionObject,
        };
        use serde_json::json;
        let system_message = "You are a helpful email assistant with the ability to summarize emails and to send Slack messages.
            You have access to the following Rust tools:
            1. `read_emails(count: usize) -> Vec<HashMap>`: Reads the top n emails from the user's mailbox.
            2. `send_slack_message(channel: String, message: String, preview: bool) -> String`: Sends a message to a Slack channel.
            3. `read_variable(variable: String) -> String`: Reads the contents of a variable to obtain results of a tool call.

            Whenever you call a tool, you will not receive the result directly. Rather, a variable standing in for the result will be appended to the conversation. You can use the `read_variable` tool to read the contents of a variable and obtain the results of the previous tool call if you MUST know it before the next tool call.

            All arguments to tools have an `anyOf` schema, with a `kind` tag indicating whether the value is a literal value (`value`) or a variable name (`variable_name`).
            When choosing tool call arguments, make sure to use the `kind` tag to indicate whether the value is a literal value or a variable name.
            - If `kind` == \"value\", the value MUST be passed in the `value` field.
            - If `kind` == \"variable\", a variable name MUST be passed in the `variable` field instead.
            Make absolutely sure to respect this convention. You MUST NOT pass a variable name in the `value` field or vice versa.

            If you are not sure about the contents of data pertaining to the user’s request, use `read_variable` or gather the relevant information from other tools: do NOT guess or make up an answer.
            The user's Slack alias is: bob.sheffield@contoso.com";
        let tools =
            vec![
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_emails_labeled".to_string(),
                    description: Some(
                        "Reading a number of {count} email from the inbox".to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "count": {
                                "type": "string",
                                "description": "The number of emails to read",
                            },
                        },
                        "required": ["count"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .build()
                .unwrap(),
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "send_slack_message_labeled".to_string(),
                    description: Some(
                        "Sends a {message} to a slack {channel} with an optional {preview}"
                            .to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "channel": {
                                "type": "string",
                                "description": "The channel where the message should be sent",
                            },
                            "message": {
                                "type": "string",
                                "description": "The message to be sent",
                            },
                            "preview": {
                                "type": "string",
                                "description": "Whether or not to include the link preview",
                            },
                        },
                        "required": ["channel", "message", "preview"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .r#type(ChatCompletionToolType::Function)
                .build()
                .unwrap(),
            ChatCompletionToolArgs::default()
                .function(FunctionObject {
                    name: "read_variable".to_string(),
                    description: Some(
                        "Read a {variable} name that save a tool result to obtain the contents"
                            .to_string(),
                    ),
                    parameters: Some(variable_schema_gen(json!({
                        "type": "object",
                        "properties": {
                            "variable": {
                                "type": "string",
                                "description": "The variable to be read",
                            },
                        },
                        "required": ["variable"],
                        "additionalProperties": false,
                    }), vec![])),
                    strict: Some(true),
                })
                .r#type(ChatCompletionToolType::Function)
                .build()
                .unwrap(),
        ];

        let tt_planner = TaintTrackingPlanner::new(tools.clone(), Policy);

        let client = LlmClient::openai();
        //let client = LlmClient::local_llama31();

        // Build a system message
        let system_request = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_message)
            .build()
            .unwrap()
            .into();
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content("Write a summary of my 5 most recent emails and send it to me as private Slack message.")
            .build()
            .unwrap()
            .into();

        let state: crate::State = crate::ConversationHistory(vec![system_request, user_message]);
        let chat_request = client.chat(state.0.clone(), tools);
        let current_message = chat_request.await.unwrap().choices[0].message.clone();

        let mut planning_loop = PlanningLoop::new(
            tt_planner,
            client,
            vec![
                MetaFunction::new("read_emails_labeled".to_string()),
                MetaFunction::new("send_slack_message_labeled".to_string()),
            ],
        );

        let email_universe: Vec<crate::tools::Email> =
            crate::tools::INBOX.iter().cloned().collect();
        // Create the address universe of all the possible addresses in the email list above
        let address_universe =
            crate::tools::EmailAddressUniverse::new(&email_universe).into_inner();
        // Create a label for the least confidentiality possible. This is basically everybody can read
        // everybody
        let least_confidentiality =
            crate::tools::readers_label(address_universe.clone(), address_universe)
                .expect("Failed to build confidentiality label for test");

        let mut datastore = crate::Datastore;
        let response = planning_loop
            .run_with_policy(
                state,
                &mut datastore,
                crate::tools::MetaValue::new(
                    Message::Chat(current_message),
                    crate::ProductLattice::new(Integrity::trusted(), least_confidentiality),
                ),
                Policy,
            )
            .await
            .expect("Failed to run");
        println!("{response:#?}");
    }
}
