use crate::{Action, Args, Datastore, Function, Message, State, openai::LlmClient};
use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool, FunctionCall, Role,
    },
};
use std::marker::PhantomData;

// Planning loop handle all interaction with the model, tools and users.
pub struct PlanningLoop<M: Clone, P: Plan<M>> {
    planner: P,
    model: LlmClient,
    tools: Vec<Function>,
    phantom: PhantomData<M>,
}

impl<P: Plan<Message>> PlanningLoop<Message, P> {
    pub fn new(planner: P, model: LlmClient, tools: Vec<Function>) -> Self {
        Self {
            planner,
            model,
            tools,
            phantom: PhantomData,
        }
    }
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub async fn run(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: Message,
    ) -> Result<String, PlanError> {
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self
                .planner
                .plan(current_state, current_message)
                .map_err(|_| PlanError::CannotPlan)?;
            match action {
                Action::Query(conv_history, tools) => {
                    let chat_request = self.model.chat(conv_history.0, tools);
                    current_message = Message::Chat(chat_request.await?.choices[0].message.clone());
                }
                Action::MakeCall(function, args) => {
                    let tool_result = self
                        .tools
                        .iter()
                        .find(|&f| f == &function)
                        .unwrap()
                        .call(args, datastore);
                    current_message = Message::ToolResult(tool_result);
                }
                Action::Finish(result) => return Ok(result),
            }
        }
    }
}


// State passing planner which is plugged into the `PlanningLoop`
pub trait Plan<M> {
    type Error;
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
                println!("Refusal {:#?}", message);
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
                                .tool_calls(tool_calls)
                                .build()?
                                .into();
                            new_state.0.push(conv_message);
                            let action = Action::MakeCall(Function(name), Args(arguments));
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
            Message::ToolResult(content) => {
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(content)
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
    CannotPlan,
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

/*
pub struct VarPlanner {
    tools: Vec<Function>,
    memory: Memory,
}

pub static ID_MANAGER: AtomicUsize = AtomicUsize::new(0);

type Memory = HashMap<Variable, ToolCallResult>;
#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Variable(String);

impl Variable {
    pub fn fresh() -> Self {
        Self(format!("{}", ID_MANAGER.fetch_add(1, Ordering::Relaxed)))
    }
}

type ToolCallResult = String;

impl Plan<Message> for VarPlanner {
    fn plan(&mut self, state: State, message: Message) -> (State, Action) {
        // We need to make available variables in memory for the next tool calls
        let tools: Vec<Function> = self
            .tools
            .iter()
            .map(|tool| tool.format_vars(self.memory.keys().collect()))
            .collect();
        // This state can also be considered as the entire conversation history
        let mut new_state = state;

        match message {
            Message::User(ref _user_message) => {
                new_state.0.push(message.clone());
                let action = Action::Query(new_state.clone(), tools);
                (new_state, action)
            }
            Message::Tool(tool_result) => {
                let x = Variable::fresh();
                self.memory.insert(x.clone(), tool_result);
                let var_message = Message::Tool(x.0);
                new_state.0.push(var_message);
                let action = Action::Query(new_state.clone(), tools);
                (new_state, action)
            }
            Message::ToolCall(ref tool, ref args) => {
                if tool.name() == "inspect" {
                    let tool_result = self
                        .memory
                        .get(&(args.0[0].clone().try_into().unwrap()))
                        .unwrap()
                        .clone();
                    new_state.0.push(Message::Tool(tool_result));
                    let action = Action::Query(new_state.clone(), tools);
                    (new_state, action)
                } else {
                    let new_args = self.expand_args(args);

                    new_state.0.push(message.clone());
                    let action = Action::MakeCall(tool.clone(), new_args);
                    (new_state, action)
                }
            }
            Message::Assistant(ref response) => {
                let action = Action::Finish(response.clone());
                new_state.0.push(message);
                (new_state, action)
            }
        }
    }
}

impl VarPlanner {
    fn expand_args(&self, args: &Args) -> Args {
        Args(
            args.0
                .iter()
                .map(|arg| match arg {
                    Arg::Basic(_basic_str) => arg.clone(),
                    Arg::Variable(var) => Arg::Basic(self.memory.get(var).unwrap().clone()),
                })
                .collect(),
        )
    }
}

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
