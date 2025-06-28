use crate::{Args, Function, Label};
use async_openai::types::ChatCompletionResponseMessage;

// A message passed as information in the planner
#[derive(Clone)]
pub enum _Message1 {
    // Represents user and system messages
    User(String),
    Tool(String),
    // Represents a model's results to call a tool `Function` with arguments specified as an array
    // of strings.
    ToolCall(Function, Args),
    // Represents a natural language response `r` from the model.
    Assistant(String),
}

// A message passed as information in the planner
pub type Message = ChatCompletionResponseMessage;

#[derive(Clone)]
pub struct LabeledMessage {
    pub message: Message,
    pub label: Label,
}
