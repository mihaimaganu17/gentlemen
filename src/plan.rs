use crate::{
    Action, Args, Datastore, Function, Message, State,
    openai::LlmClient,
    tools::{Memory, Variable},
};
use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool, FunctionCall, Role,
    },
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Planning loop orchestrates the communication with the model and handles the `Planner`'s
/// required actions.
pub struct PlanningLoop<M: Clone, P: Plan<M>> {
    // The planner used to plan the next action in the loop
    planner: P,
    // The LLM model used to accomplish the task
    model: LlmClient,
    // The tools the LLM model has access to
    tools: Vec<Function>,
    // Phantom data such that we can bind the type of `Message` that the planner `P` uses
    phantom: PhantomData<M>,
}

impl<P: Plan<Message>> PlanningLoop<Message, P> {
    /// Create a new `PlanninLoop` with an action `planner` a `model` to do the work and available
    /// `tools` that the model can call
    pub fn new(planner: P, model: LlmClient, tools: Vec<Function>) -> Self {
        Self {
            planner,
            model,
            tools,
            phantom: PhantomData,
        }
    }
    /// The entry point for executing the `PlanningLoop`. At each iteration of the loop, the
    /// current `state`, the latest `message` of the conversation and the `datastore` are passed.
    pub async fn run(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: Message,
    ) -> Result<String, PlanError> {
        // Bind the given message to a mutable variable as it will be updated inside the following
        // loop based on what action the loop is taking.
        let mut current_message = message;
        // Bind the given state to a mutable variable as it will be updates insied the following
        // loop with a new message.
        let mut current_state = state;
        loop {
            let action;
            // Plan the next action giving the current message and state. The new message is sent
            // separate from the state as it will be converted by the planner from a
            // `ChatCompletionRequest{Type}` message to a `ChatCompletionResponse{Type}` message.
            (current_state, action) = self
                .planner
                .plan(current_state, current_message)
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            match action {
                // We have to query the model
                Action::Query(conv_history, tools) => {
                    // Build a chat request with all the previous conversation history and the
                    // available tools
                    let chat_request = self.model.chat(conv_history.0, tools);
                    // Send the request and save the first response choice as the new message
                    current_message = Message::Chat(chat_request.await?.choices[0].message.clone());
                }
                // We have to call a tool requested by the model
                Action::MakeCall(function, args, id) => {
                    // Find the requested `function` and call it with the given arguments
                    let tool_result = self
                        .tools
                        .iter()
                        .find(|&f| f == &function)
                        .unwrap()
                        .call(args, datastore);
                    // New message represents the result we got from calling the above tool and we
                    // also keep the tool id such that the model can associate the tools request
                    // with the tool id.
                    current_message = Message::ToolResult(tool_result, id);
                }
                // We got the final model response and we return it back to the caller
                Action::Finish(result) => return Ok(result),
            }
        }
    }
}

// State passing planner which is plugged into the `PlanningLoop`
pub trait Plan<M> {
    type Error: std::fmt::Debug;
    fn plan(&mut self, state: State, message: M) -> Result<(State, Action), Self::Error>;
}

pub struct BasicPlanner {
    tools: Vec<ChatCompletionTool>,
}

impl BasicPlanner {
    pub fn new(tools: Vec<ChatCompletionTool>) -> Self {
        Self { tools }
    }
}

impl Plan<Message> for BasicPlanner {
    type Error = PlanError;
    fn plan(&mut self, state: State, message: Message) -> Result<(State, Action), Self::Error> {
        let mut new_state = state;
        let (new_state, action) = match message {
            Message::Chat(message) => {
                let role = message.role;
                match role {
                    Role::User => {
                        let conv_message = ChatCompletionRequestUserMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoUserContent)?)
                            .build()?
                            .into();
                        new_state.0.push(conv_message);
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Tool => {
                        let conv_message = ChatCompletionRequestToolMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoToolContent)?)
                            .tool_call_id(message.tool_calls.unwrap()[0].id.clone())
                            .build()?
                            .into();
                        new_state.0.push(conv_message);
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Assistant => {
                        if let Some(tool_calls) = message.tool_calls {
                            let FunctionCall { name, arguments } = tool_calls[0].clone().function;
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .tool_calls(vec![tool_calls[0].clone()])
                                .build()?
                                .into();
                            new_state.0.push(conv_message);
                            let action = Action::MakeCall(
                                Function(name),
                                Args(arguments),
                                tool_calls[0].clone().id,
                            );
                            (new_state, action)
                        } else if let Some(content) = message.content {
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into();
                            new_state.0.push(conv_message);
                            let action = Action::Finish(content);
                            (new_state, action)
                        } else {
                            todo!();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            Message::ToolResult(content, id) => {
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(content)
                    .tool_call_id(id)
                    .build()?
                    .into();
                new_state.0.push(conv_message);
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
        };
        Ok((new_state, action))
    }
}

#[derive(Debug)]
pub enum PlanError {
    NoUserContent,
    NoToolContent,
    NoToolCalls,
    NoFunctionCall,
    CannotPlan(String),
    OpenAIError(OpenAIError),
}

impl From<OpenAIError> for PlanError {
    fn from(err: OpenAIError) -> Self {
        Self::OpenAIError(err)
    }
}

/*
impl<P: Plan<LabeledMessage>> PlanningLoop<LabeledMessage, P> {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub fn run(
        &mut self,
        _state: State,
        _datastore: &mut Datastore,
        _message: LabeledMessage,
    ) -> String {
        todo!()
    }

    pub fn run_with_policy(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: LabeledMessage,
        policy: Policy,
    ) -> String {
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self.planner.plan(current_state, current_message.clone());
            match action {
                Action::Query(conv_history, tools) => {
                    /*
                    let new_message = self.model.map(conv_history, tools);
                    // TODO: Create label for this message
                    current_message = LabeledMessage {
                        message: new_message,
                        label: Label,
                    }
                    */
                    todo!(),
                }
                Action::MakeCall(ref function, ref args) => {
                    // Here both `function` and `args` have a label
                    if !policy.is_allowed(&action) {
                        // Do not perform the action
                        continue;
                    }
                    let tool_result = self
                        .tools
                        .iter()
                        .find(|&f| f == function)
                        .unwrap()
                        .call(args.clone(), datastore);
                    // TODO: Create label for this message
                    current_message = LabeledMessage {
                        message: tool_result,
                        label: Label,
                    }
                }
                Action::Finish(result) => return result,
            }
        }
    }
}
*/

pub struct VarPlanner {
    tools: Vec<ChatCompletionTool>,
    memory: Memory,
}

impl VarPlanner {
    pub fn new(tools: Vec<ChatCompletionTool>) -> Self {
        Self {
            tools,
            memory: HashMap::new(),
        }
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

impl Plan<Message> for VarPlanner {
    type Error = PlanError;
    fn plan(&mut self, state: State, message: Message) -> Result<(State, Action), Self::Error> {
        let mut new_state = state;
        println!("message {:#?}", message);
        let (new_state, action) = match message {
            Message::Chat(message) => {
                let role = message.role;
                match role {
                    Role::User => {
                        let conv_message = ChatCompletionRequestUserMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoUserContent)?)
                            .build()?
                            .into();
                        new_state.0.push(conv_message);
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Tool => {
                        let x = Variable::fresh();
                        self.memory.insert(x.clone(), message.content.unwrap());
                        let conv_message = ChatCompletionRequestToolMessageArgs::default()
                            .content(x.value)
                            .tool_call_id(message.tool_calls.unwrap()[0].id.clone())
                            .build()?
                            .into();
                        new_state.0.push(conv_message);
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Assistant => {
                        if let Some(ref tool_calls) = message.tool_calls {
                            let FunctionCall { name, arguments } = tool_calls[0].clone().function;
                            let action = if name == "read_variable" {
                                let variable = self.normalize_args(arguments);
                                let result = self.memory.get(&serde_json::from_str(&variable).unwrap()).unwrap();
                                let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                    .tool_calls(vec![tool_calls[0].clone()])
                                    .build()?
                                    .into();
                                new_state.0.push(conv_message);
                                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                                    .content(result.clone())
                                    .tool_call_id(message.tool_calls.unwrap()[0].id.clone())
                                    .build()?
                                    .into();
                                new_state.0.push(conv_message);
                                let action = Action::Query(new_state.clone(), self.tools.clone());
                                action
                            } else {
                                let conv_message =
                                    ChatCompletionRequestAssistantMessageArgs::default()
                                        .tool_calls(vec![tool_calls[0].clone()])
                                        .build()?
                                        .into();
                                new_state.0.push(conv_message);
                                let action = Action::MakeCall(
                                    Function(name),
                                    Args(self.normalize_args(arguments)),
                                    tool_calls[0].clone().id,
                                );
                                action
                            };
                            (new_state, action)
                        } else if let Some(content) = message.content {
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into();
                            new_state.0.push(conv_message);
                            let action = Action::Finish(content);
                            (new_state, action)
                        } else {
                            todo!();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            Message::ToolResult(content, id) => {
                println!("{content:#?} {id:#?}");
                let x = Variable::fresh();
                self.memory.insert(x.clone(), content);
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(x.value)
                    .tool_call_id(id)
                    .build()?
                    .into();
                new_state.0.push(conv_message);
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
        };
        Ok((new_state, action))
    }
}

/*
pub struct TaintTrackingPlanner {
    tools: Vec<Function>,
    memory: Memory,
}

// Taint-tracking planner which is plugged into the `PlanningLoop`
impl Plan<LabeledMessage> for TaintTrackingPlanner {
    fn plan(&mut self, state: State, message: LabeledMessage) -> (State, Action) {
        todo!()
    }
}
*/
