mod basic;
mod plan_loop;
mod var;
mod labeled;

pub use basic::BasicPlanner;
pub use plan_loop::PlanningLoop;
pub use var::VarPlanner;

use crate::Action;
use async_openai::error::OpenAIError;
use serde_json::Value;

/// Enables a state passing planner which is plugged into the `PlanningLoop`
pub trait Plan<S, M> {
    type Error: std::fmt::Debug;
    /// Take and process a previous known `state` and the current `message` and returns a new state
    /// which contains the previous message and an action to be taken by the caller.
    fn plan(&mut self, state: S, message: M) -> Result<(S, Action), Self::Error>;
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
