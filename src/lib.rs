pub mod ifc;
mod message;
pub mod openai;
mod plan;
pub mod tools;

pub use ifc::{Confidentiality, Integrity, Label, ProductLattice};
pub use message::{LabeledMessage, Message};
pub use plan::{BasicPlanner, Plan, PlanningLoop, VarPlanner};
use std::fmt;
use tools::{ReadEmailsArgs, SendSlackMessageArgs, read_emails, send_slack_message};

// use plan::Variable;
use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionTool};

pub struct Datastore;

// This should also be a trait
#[derive(PartialEq, Clone)]
pub struct Function(String);

impl Function {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    pub fn call(&self, args: Args, _datastore: &mut Datastore) -> String {
        match self.0.as_str() {
            "read_emails" => {
                // Convert args to desired type
                let args: ReadEmailsArgs = serde_json::from_str(&args.0).unwrap();
                let result = read_emails(args);
                println!("{result:?}");
                serde_json::to_string(&result).unwrap()
            }
            "send_slack_message" => {
                let args: SendSlackMessageArgs = serde_json::from_str(&args.0).unwrap();
                let result = send_slack_message(args);
                println!("{result:?}");
                serde_json::to_string(&result).unwrap()
            }
            _ => panic!("{:?}", self.0),
        }
    }
}

#[derive(Clone)]
pub struct Args(pub String);

#[derive(Clone)]
pub enum Arg {
    Basic(String),
    //Variable(Variable),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(value) => write!(f, "{}", value),
        }
    }
}

// Comprises all the messages in the conversation up to the current point
#[derive(Clone)]
pub struct ConversationHistory<T>(Vec<T>);
type State = ConversationHistory<ChatCompletionRequestMessage>;

#[derive(Debug)]
pub enum ConversionError {
    ArgIsNotVariable,
}

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
