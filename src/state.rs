use crate::Label;
use async_openai::types::ChatCompletionRequestMessage;

// Comprises all the messages in the conversation up to the current point
#[derive(Clone)]
pub struct ConversationHistory<T>(pub Vec<T>);
pub type State = ConversationHistory<ChatCompletionRequestMessage>;

#[derive(Clone)]
pub struct LabeledConversationHistory<M> {
    conv: Vec<M>,
    label: Label,
}

impl<M> LabeledConversationHistory<M> {
    pub fn new(conv: Vec<M>, label: Label) -> Self {
        Self { conv, label }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }
}

pub type LabeledState = LabeledConversationHistory<ChatCompletionRequestMessage>;

impl LabeledState {
    pub fn messages(&self) -> &[ChatCompletionRequestMessage] {
        self.conv.as_ref()
    }
}
