struct State;

// A message passed as information in the planner
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
struct Function;

impl Function {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Args, datastore: Datastore) -> (Datastore, Message) {
        (datastore, Message::Assistant("I have no tools"))
    }
}

struct Args(Vec<String>)

enum Action {
    // Query the model with a specific conversation history and available tools
    Query(CoversationHistor, Vec<Function>),
    // Call a `Tool` with `Args`
    MakeCall(Function, Args),
    // Finish the conversation and respond to the user.
    Finish(String),
}

// Comprises all the messages in the conversation up to the current point
struct ConversationHistory(Vec<Message>)

// Planning loop handle all interaction with the model, tools and users.
struct PlanningLoop {
    planner: Planner,
    model: Model,
    tools: Vec<Function>,
}

impl PlanningLoop {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    fn loop(state: State, datastore: Datastore, message: Message) -> String {
        let mut current_message = message;
        loop {
            let (state, action) = self.planner(state, message);
            match action {
                Action::Query(conv_history, tools) => {
                    let new_message = self.model.map(conv_history, tools)
                    current_message = new_message
                }
                Action::MakeCall(function, args) => {
                    let (new_datastore, tool_result) = self.tools.get(function).call(args, datastore);
                    datatore = new_datastore;
                    current_message = Message::Tool(tool_result);
                }
                Action::Finish(result) => return result
            }
        }
    }
}

// State passing planner which is plugged into the `PlanningLoop`
enum Planner {
    Basic,
    Variable,
}

impl Planner {
    fn loop(&self, state: State) {
        match self {
            Planner::Basic => self.basic_planner(state),
            Planner::Variable => self.var_planner(state),
        }
    }

    fn basic_planner(&self, state: State, message: Message) -> (State, Action){
        let new_state = match message {

        }
    }
}

// Model is a mapping between a sequence of messages and tool declarations to either a tool call or
// a response. This should be a trait
struct Model;

impl Model {
    fn map(conv_history: ConversationHistory, tools: Vec<Function>) -> Message {
        // This should be either a tool call or an Assitant message
        Message::Assistant("I have no idea what I am doing")
    }
}

#[cfg(test)]
mod tests {
}
