pub mod function;
pub mod ifc;
mod message;
pub mod openai;
mod plan;
mod state;
pub mod tools;

pub use function::{Args, Call, Function, LabeledArgs, LabeledFunction};
pub use ifc::{Confidentiality, Integrity, Label, ProductLattice};
pub use message::{LabeledMessage, Message};
pub use plan::{BasicPlanner, Plan, PlanningLoop, Policy, TaintTrackingPlanner, VarPlanner};
pub use state::{ConversationHistory, LabeledConversationHistory, LabeledState, State};

// use plan::Variable;
use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionTool};

pub struct Datastore;

#[derive(Debug)]
pub enum Action {
    // Query the model with a specific conversation history and available tools
    Query(
        ConversationHistory<ChatCompletionRequestMessage>,
        Vec<ChatCompletionTool>,
    ),
    // Call a `Tool` with `Args`
    MakeCall(Function, Args, String),
    // Finish the conversation and respond to the user.
    Finish(String),
}

pub enum TaskType {
    DataDependent,
    DataIndependent,
}

pub struct Task {
    _query: String,
    _tools: Vec<Function>,
    _datastores: Vec<Datastore>,
}

#[cfg(test)]
mod tests {}
