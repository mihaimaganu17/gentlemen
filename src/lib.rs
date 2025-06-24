mod message;
mod plan;

pub use message::{Message, LabeledMessage};
use plan::Variable;
pub use plan::{Plan, PlanningLoop};

pub struct Datastore;

// This should also be a trait
#[derive(PartialEq, Clone)]
pub struct Function;

impl Function {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    pub fn call(&self, _args: Args, _datastore: &mut Datastore) -> Message {
        Message::Assistant("I have no tools".to_string())
    }

    fn format_vars(&self, _variables: Vec<&Variable>) -> Self {
        todo!()
    }

    fn name(&self) -> &str {
        "Anonym"
    }
}

#[derive(Clone)]
pub struct Args(Vec<Arg>);

#[derive(Clone)]
pub enum Arg {
    Basic(String),
    Variable(Variable),
}

impl TryFrom<Arg> for Variable {
    type Error = ConversionError;

    fn try_from(value: Arg) -> Result<Self, Self::Error> {
        match value {
            Arg::Variable(var) => Ok(var),
            _ => Err(ConversionError::ArgIsNotVariable),
        }
    }
}

#[derive(Debug)]
pub enum ConversionError {
    ArgIsNotVariable,
}

pub enum Action {
    // Query the model with a specific conversation history and available tools
    Query(ConversationHistory, Vec<Function>),
    // Call a `Tool` with `Args`
    MakeCall(Function, Args),
    // Finish the conversation and respond to the user.
    Finish(String),
}

// Comprises all the messages in the conversation up to the current point
#[derive(Clone)]
pub struct ConversationHistory(Vec<Message>);
type State = ConversationHistory;

// Model is a mapping between a sequence of messages and tool declarations to either a tool call or
// a response. This should be a trait
pub struct Model;

impl Model {
    pub fn map(&self, _conv_history: ConversationHistory, _tools: Vec<Function>) -> Message {
        // This should be either a tool call or an Assitant message
        Message::Assistant("I have no idea what I am doing".to_string())
    }
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

enum _Confidentiality {
    High(_Secret),
    Low(_Public),
}

// Public <= Secret (partial order)
struct _Public;
struct _Secret;

enum _Integrity {
    Trusted(_HighIntegrity),
    Untrusted(_LowIntegrity),
}

struct _HighIntegrity;
struct _LowIntegrity;

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
