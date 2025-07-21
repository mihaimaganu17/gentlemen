pub mod ifc;
mod message;
mod state;
pub mod openai;
mod plan;
pub mod tools;
pub mod function;

pub use ifc::{Confidentiality, Integrity, Label, ProductLattice};
pub use message::{LabeledMessage, Message};
pub use plan::{BasicPlanner, Plan, PlanningLoop, VarPlanner};
pub use state::{State, LabeledState, ConversationHistory, LabeledConversationHistory};
pub use function::{Function, LabeledFunction, Args, LabeledArgs, Call};
use std::fmt;

// use plan::Variable;
use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionTool};

pub struct Datastore;

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

// Planners get instrumented with dynamic information-flow control via taint-tracking. For this,
// labels are attached to messages, actions, tool arguments and results, and vairables in the
// datastore.
//
// Labels originate from data read by tools from the datastore, which tools propagate to their
// results,
// the planner propagates from messages to actions
// and the planning loop propagates throughout its execution.
//
// We add a metadata field to label each node in the syntax tree of tools results.
// When non-empty, such a label applied to all field of that node and below.
//
// Also attach metadata field to label individual messages in the conversation history.
// The initial system and user messages are typically considered trusted and public and by default.

// A trace is a sequence of actions that the model takes starting from a user's Message::Query
// and ending with an `Action::Finish`.
pub struct Trace(pub Vec<Action>);

#[cfg(test)]
mod tests {}
