mod plan;

use plan::Variable;

struct Datastore;

// A message passed as information in the planner
#[derive(Clone)]
enum Message {
    // Represents user and system messages
    User(String),
    Tool(String),
    // Represents a model's resuts to call a tool `Function` with arguments specified as an array
    // of strings.
    ToolCall(Function, Args),
    // Represents a natural language response `r` from the model.
    Assistant(String),
}

// This should also be a trait
#[derive(PartialEq, Clone)]
struct Function;

impl Function {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Args, datastore: &mut Datastore) -> Message {
        Message::Assistant("I have no tools".to_string())
    }

    fn format_vars(&self, variables: Vec<&Variable>) -> Self {
        todo!()
    }
}


#[derive(Clone)]
struct Args(Vec<Arg>);

#[derive(Clone)]
enum Arg {
    Basic(String),
    Variable(Variable),
}

enum Action {
    // Query the model with a specific conversation history and available tools
    Query(ConversationHistory, Vec<Function>),
    // Call a `Tool` with `Args`
    MakeCall(Function, Args),
    // Finish the conversation and respond to the user.
    Finish(String),
}

// Comprises all the messages in the conversation up to the current point
#[derive(Clone)]
struct ConversationHistory(Vec<Message>);
type State = ConversationHistory;

// Model is a mapping between a sequence of messages and tool declarations to either a tool call or
// a response. This should be a trait
struct Model;

impl Model {
    fn map(&self, conv_history: ConversationHistory, tools: Vec<Function>) -> Message {
        // This should be either a tool call or an Assitant message
        Message::Assistant("I have no idea what I am doing".to_string())
    }
}

#[cfg(test)]
mod tests {
}
