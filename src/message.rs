use crate::{Args, Function, Label};

// A message passed as information in the planner
#[derive(Clone)]
pub enum Message {
    // Represents user and system messages
    User(String),
    Tool(String),
    // Represents a model's resuts to call a tool `Function` with arguments specified as an array
    // of strings.
    ToolCall(Function, Args),
    // Represents a natural language response `r` from the model.
    Assistant(String),
}

#[derive(Clone)]
pub struct LabeledMessage {
    pub message: Message,
    pub label: Label,
}
