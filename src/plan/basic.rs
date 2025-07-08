use super::{Plan, PlanError};
use crate::{Action, Args, Function, Message, State};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, FunctionCall, Role,
};
use serde_json::{Map, Value};

/// A planner that takes a set of actions given an arraty of tools
pub struct BasicPlanner {
    tools: Vec<ChatCompletionTool>,
}

impl BasicPlanner {
    /// Create a new [`BasicPlanner`] given an array of `tools`
    pub fn new(tools: Vec<ChatCompletionTool>) -> Self {
        Self { tools }
    }

    pub fn normalize_args(&self, args: String) -> String {
        let args = serde_json::from_str(&args).unwrap();
        let Value::Object(map) = args else {
            return "Mata".to_string();
        };
        let mut new_args = Map::new();

        for (arg_name, value) in map.into_iter() {
            match value {
                Value::Object(kind_map) => {
                    match kind_map.get("kind").unwrap().as_str() {
                        Some("value") => {
                            new_args.insert(arg_name, kind_map.get("value").unwrap().clone())
                        }
                        Some("variable") => todo!(),
                        Some(kind) => panic!("{}", format!("Invalid kind argument {kind}")),
                        None => panic!("kind field is missing"),
                    };
                }
                _ => panic!("Invalid argument schema {value:#?}"),
            }
        }
        serde_json::to_string(&Value::Object(new_args)).unwrap()
    }
}

impl Plan<Message> for BasicPlanner {
    type Error = PlanError;

    /// Take and process a previous known `state` and the current `message` and returns a new state
    /// which contains the previous message and an action to be taken by the caller.
    fn plan(&mut self, state: State, message: Message) -> Result<(State, Action), Self::Error> {
        // Bind the state to a mutable state such that we can update it.
        let mut new_state = state;

        // Create a new state and action based on the message we get. This match also converts
        // the message from a completion response type message to a completion request type message.
        let (new_state, action) = match message {
            // If we have a chat message between the user and the assistant.
            Message::Chat(message) => {
                // Get the role of the message
                let role = message.role;
                // Convert the message and create a new action depending on the role
                match role {
                    Role::User => {
                        // For user messages we only care about the content
                        let conv_message = ChatCompletionRequestUserMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoUserContent)?)
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);
                        // In this case, the action to take is to query the LLM with the updated
                        // state and the set of available tools
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Tool => {
                        // For tools messages we want to capture the content of the tool aka the
                        // result that the tool sent back and the tool's id, such that the LLM
                        // can match the tool call with the tool result.
                        let conv_message = ChatCompletionRequestToolMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoToolContent)?)
                            .tool_call_id(message.tool_calls.unwrap()[0].id.clone())
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);

                        // In this case, the action to take is to query the LLM with the updated
                        // state and the set of available tools
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Assistant => {
                        // If we have an assistant message, our response depends on whether the
                        // message is a tool call or a pure chat message.

                        // In the case of a tool call.
                        if let Some(tool_calls) = message.tool_calls {
                            // Currently there is no support for multiple tool calls in one
                            // message.
                            assert!(tool_calls.len() == 1);
                            // Get the name and argument of the first tool call.
                            let FunctionCall { name, arguments } = tool_calls[0].clone().function;

                            // Normalize arguments such that we could parse them in their correct
                            // function input
                            let arguments = self.normalize_args(arguments);

                            // Convert the message to a request to update the state
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .tool_calls(vec![tool_calls[0].clone()])
                                .build()?
                                .into();
                            // Update the state with the new message
                            new_state.0.push(conv_message);

                            // In this case, the action to take is to call the specified tool with
                            // the specified arguments, keeping the id of the tool call such that
                            // we can report it back to the LLM in the message that will contain
                            // the tool result.
                            let action = Action::MakeCall(
                                Function(name),
                                Args(arguments),
                                tool_calls[0].clone().id,
                            );
                            (new_state, action)
                        // In the case of an assitant pure chat message
                        } else if let Some(content) = message.content {
                            // Convert the message response into a request and copy over the
                            // contents of the message
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into();
                            // Update the state with the new message
                            new_state.0.push(conv_message);
                            // In this case, the assistant gave the "final" answer as we want to
                            // take a finishing action and return the result to the caller.
                            let action = Action::Finish(content);
                            (new_state, action)
                        } else {
                            todo!();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            // If we have a tool result, we are in a similar case with the chat message in the tool
            // role above. However this is separate since this type of message is generated by the
            // current process and not by the LLM in order to fill it with a tool result.
            Message::ToolResult(content, id) => {
                // Convert the message to a request to update the state
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(content)
                    .tool_call_id(id)
                    .build()?
                    .into();
                // Update the state with the new message
                new_state.0.push(conv_message);

                // In this case, the action to take is to query the LLM with the updated
                // state and the set of available tools
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
        };
        Ok((new_state, action))
    }
}
