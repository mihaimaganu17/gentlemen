mod basic;
mod plan_loop;
mod var;

pub use basic::BasicPlanner;
pub use plan_loop::PlanningLoop;
pub use var::VarPlanner;

use crate::{Action, State};
use async_openai::error::OpenAIError;
use serde_json::Value;

/// Enables a state passing planner which is plugged into the `PlanningLoop`
pub trait Plan<M> {
    type Error: std::fmt::Debug;
    /// Take and process a previous known `state` and the current `message` and returns a new state
    /// which contains the previous message and an action to be taken by the caller.
    fn plan(&mut self, state: State, message: M) -> Result<(State, Action), Self::Error>;
}

/// Error issued by either one of the planners which implement [`Plan`] or the [`PlanningLoop`]
#[derive(Debug)]
pub enum PlanError {
    NoUserContent,
    NoToolContent,
    NoToolCalls,
    NoFunctionCall,
    CannotPlan(String),
    OpenAIError(OpenAIError),
    ArgumentNotObject(Value),
    SerdeJsonError(serde_json::Error),
    InvalidObjectKey(String),
    InvalidArgumentKind(String),
    ArgumentMissingKind(String),
    InvalidArgumentSchema(Value),
    InvalidMessage(String),
    MissingVariable(String),
}

impl From<OpenAIError> for PlanError {
    fn from(err: OpenAIError) -> Self {
        Self::OpenAIError(err)
    }
}

impl From<serde_json::Error> for PlanError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJsonError(err)
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
        ?}
    }
}
*/

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
