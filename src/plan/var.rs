use super::{PlanError, Plan};
use crate::{
    Action, Args, Function, Message, State,
    tools::{Memory, Variable},
};
use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool, FunctionCall, Role,
    },
};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// A planner that takes a set of actions given an array of tools. It does not returns tool results
/// directly to the LLM, but rather it uses internal `memory` to map tool results to variables and
/// then when queried about a variable ID, it returns the matching tool result
pub struct VarPlanner {
    // Set of tools the LLM could choose to call.
    tools: Vec<ChatCompletionTool>,
    // Memory mapping variable names to tool results from tool calls
    memory: Memory,
}

impl VarPlanner {
    /// Create a new [`VarPlanner`] with the given `tools` and empty memory
    pub fn new(tools: Vec<ChatCompletionTool>) -> Self {
        Self {
            tools,
            memory: HashMap::new(),
        }
    }

    /// Normalize the arguments passed by the LLM. The LLM is instructed to pass a specific schema
    /// for the function arguments such that it could be distinguished which arguments are
    /// `variables` which have to be queried by internal memory and which are plain variables which
    /// only need to be passed to the function call. Each argument type is specified in the `kind`
    /// field and the `value` field holds the actual value of the argument
    pub fn normalize_args(&self, args: String) -> Result<String, PlanError> {
        // Convert the arguments to a [`serder_json::Value`]
        let args = serde_json::from_str(&args)?;

        // If the arguments are not an object, in other words a json dictionary
        let Value::Object(map) = args else {
            // We do not support it and return an error
            return Err(PlanError::ArgumentNotObject(args));
        };

        // Create a new [`Map`] that will hold the arguments in their normalized form
        let mut new_args = Map::new();

        // For each argument
        for (arg_name, value) in map.into_iter() {
            match value {
                // If we have another map representing the argument
                Value::Object(kind_map) => {
                    // Check its kind
                    match kind_map
                        .get("kind")
                        .ok_or(PlanError::InvalidObjectKey("kind".to_string()))?
                        .as_str()
                    {
                        // If it is a value we take the value as is
                        Some("value") => new_args.insert(
                            arg_name,
                            kind_map
                                .get("value")
                                .ok_or(PlanError::InvalidObjectKey("value".to_string()))?
                                .clone(),
                        ),
                        // If it is a variable, we need to query it in the internal [`Memory`].
                        // However this is an interesting case as currently the LLM does not listen
                        // to our instructions and never returns a `kind: variable` value.
                        Some("variable") => todo!(),
                        // Any other kind value is an error
                        Some(kind) => return Err(PlanError::InvalidArgumentKind(kind.to_string())),
                        // If the kind field is missing, we return an error
                        None => return Err(PlanError::ArgumentMissingKind(arg_name)),
                    };
                }
                // If the argument schema is no a map (dict) we consider it invalid
                _ => return Err(PlanError::InvalidArgumentSchema(value)),
            }
        }

        // Convert the new map into a string and return it
        Ok(serde_json::to_string(&Value::Object(new_args))?)
    }
}

impl Plan<Message> for VarPlanner {
    type Error = PlanError;
    fn plan(&mut self, state: State, caller_message: Message) -> Result<(State, Action), Self::Error> {
        // TODO: Move these printlns to a logging module
        println!("{:#?}", caller_message);
        println!("{:#?}", self.memory);

        // Make the passed state mutable such that we can update it with the new message
        let mut new_state = state;
        // Based on the type of message passed in by the caller, we take an action
        let (new_state, action) = match caller_message {
            // A chat message between the user and the assitant
            Message::Chat(message) => {
                let role = message.role;
                // Depending on the role of the message
                match role {
                    // If it was a user message
                    Role::User => {
                        // Convert it to a request (from a response) with the same content
                        let conv_message = ChatCompletionRequestUserMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoUserContent)?)
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);
                        // In this case we query the model with all the updated state and the
                        // tools.
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    // If it was a tool message (the result of a tool), map the result to an
                    // internal variable and return the variable
                    Role::Tool => {
                        // Generate a new variable
                        let x = Variable::fresh();
                        // Insert the new message's content mapped to the variable
                        self.memory.insert(x.clone(), message.content.ok_or(PlanError::NoToolContent)?);
                        // Create a tool message with the variable name as the content and the tool
                        // id (matching the requested tool we just called). The model will be
                        // instructed to inspect this variable and will get back the data backing
                        // it
                        let conv_message = ChatCompletionRequestToolMessageArgs::default()
                            .content(x.value)
                            .tool_call_id(message.tool_calls.ok_or(PlanError::NoToolCalls)?[0].id.clone())
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);
                        // In this case we query the model with all the updated state and the
                        // tools.
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    // If it was an assistant message we have 3 cases which involve content and
                    // tool calls and the type of tool.
                    Role::Assistant => {
                        // We get a tool call
                        if let Some(ref tool_calls) = message.tool_calls {
                            // Currently only one tool call per message is supported
                            assert!(tool_calls.len() == 1);
                            // Destruct the tool call's function
                            let FunctionCall { name, arguments } = tool_calls[0].clone().function;
                            // If the tool call corresponds to the `read_variable` function, we
                            // need to handle this special case here instead of sending back and
                            // `Action` to the caller to call the tool.
                            // We will take the variable requested as argument by the LLM and give
                            // back the tool result that it maps too.
                            let action = if name == "read_variable" {
                                // Convert LLM communication arguments to the tool's arguments,
                                // which is a variable's name.
                                let variable = self.normalize_args(arguments)?;
                                // Get the variable's corresponding tool result from the internal
                                // memory
                                let result = self
                                    .memory
                                    .get(&serde_json::from_str(&variable)?)
                                    .ok_or(PlanError::MissingVariable(variable))?;
                                // Convert the tool call message from the assistant to a request
                                // message with the tool call's contents
                                let conv_message =
                                    ChatCompletionRequestAssistantMessageArgs::default()
                                        .tool_calls(vec![tool_calls[0].clone()])
                                        .build()?
                                        .into();
                                // Update the state with the message
                                new_state.0.push(conv_message);
                                // Build another tool role message which contains the tool results
                                // that were mapped to the variable's name we got as argument. Also
                                // add the tool call id generated by the LLM.
                                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                                    .content(result.clone())
                                    .tool_call_id(message.tool_calls.ok_or(PlanError::NoToolCalls)?[0].id.clone())
                                    .build()?
                                    .into();
                                // Update the state with this tool result message
                                new_state.0.push(conv_message);
                                // In this case we query the LLM with the 2 newly constructed
                                // messages
                                Action::Query(new_state.clone(), self.tools.clone())
                            // If the tool call is not the `read_variable` tool
                            } else {
                                // We convert the message to a request message to be able to send
                                // it back
                                let conv_message =
                                    ChatCompletionRequestAssistantMessageArgs::default()
                                        .tool_calls(vec![tool_calls[0].clone()])
                                        .build()?
                                        .into();
                                // Update the state with the new message
                                new_state.0.push(conv_message);
                                // Create an `Action` which instructs the caller to call the
                                // function `name` with the normalized `arguments` and the LLM
                                // generated tool id.
                                Action::MakeCall(
                                    Function(name),
                                    Args(self.normalize_args(arguments)?),
                                    tool_calls[0].clone().id,
                                )
                            };
                            (new_state, action)
                        // If the message does not contain a tool call, but rather content
                        } else if let Some(content) = message.content {
                            // Convert the response to a request such that we can add it to the
                            // conversation history
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into();
                            // Update the state with the new message
                            new_state.0.push(conv_message);
                            // Return a finishing `Action` to the caller, instructing that the 
                            // LLM gave the final response.
                            let action = Action::Finish(content);
                            (new_state, action)
                        } else {
                            return Err(PlanError::InvalidMessage(format!("{:#?}", message)));
                        }
                    }
                    _ => return Err(PlanError::InvalidMessage(format!("{:#?}", message))),
                }
            }
            // If the message sent by the caller of this function is not a chat message between
            // the user and the assistant, but rather a tool result generated by the caller itself
            // by calling a tool.
            Message::ToolResult(content, id) => {
                // We generate a new unique identifier for a new variable
                let x = Variable::fresh();
                // Insert the contents of the tool result in the internal memory, having the
                // variable's name as key.
                self.memory.insert(x.clone(), content);
                // We convert this caller only message into a tool result message to be sent to the
                // LLM containing the name of the variable mapping this tool result and the tool
                // id that was generated in a previous assistant's tool call message
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(x.value)
                    .tool_call_id(id)
                    .build()?
                    .into();
                // Update the state with the newly generated message
                new_state.0.push(conv_message);
                // In this case, we query the model with the conversation history which now also
                // has the variable corresponding to the requested tool call
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
        };
        Ok((new_state, action))
    }
}
