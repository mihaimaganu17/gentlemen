use crate::{Datastore, Plan, PlanningLoop, Function, Arg, ConversationHistory, Action, Message,
    Confidentiality, Integrity, ProductLattice, Label, LabeledMessage, Args,
    plan::PlanError,
};
use async_openai::types::ChatCompletionRequestMessage;

#[derive(Clone)]
pub struct LabeledConversationHistory<M> {
    conv: ConversationHistory<M>,
    label: Label,
}

type LabeledState = LabeledConversationHistory<ChatCompletionRequestMessage>;

pub enum LabeledAction {
    // Query the model with a specific conversation history and available tools
    Query(LabeledState, Vec<LabeledFunction>),
    // Call a `Tool` with `Args`
    MakeCall(LabeledFunction, LabeledArgs),
    // Finish the conversation and respond to the user.
    Finish(String),
}

#[derive(Clone)]
pub struct LabeledArgs(Vec<LabeledArg>);

#[derive(Clone)]
pub struct LabeledArg {
    arg: Arg,
    label: Label,
}

// This should also be a trait
#[derive(PartialEq, Clone)]
pub struct LabeledFunction {
    function: Function,
    label: Label,
}

pub struct Policy;

impl Policy {
    fn is_allowed(&self, _action: &LabeledAction) -> bool {
        true
    }
}

impl LabeledFunction {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    pub fn call(&self, args: LabeledArgs, datastore: &mut Datastore) -> String {
        self.function.call(Args(args.0.iter().map(|x| x.arg.to_string()).collect()), datastore)
    }
}

impl<P: Plan<LabeledState, LabeledMessage>> PlanningLoop<LabeledState, LabeledMessage, P> {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub fn run_with_policy(
        &mut self,
        state: LabeledState,
        datastore: &mut Datastore,
        message: LabeledMessage,
        policy: Policy,
    ) -> Result<String, PlanError> {
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self.planner_mut().plan(current_state, current_message.clone())
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            match action {
                Action::Query(conv_history, tools) => {
                    /*
                    let new_message = self.model.map(conv_history, tools);
                    // TODO: Create label for this message
                    current_message = LabeledMessage {
                        message: new_message,
                        label: Label,
                    }
                    */
                    todo!()
                }
                Action::MakeCall(ref function, ref args, id) => {
                    // Here both `function` and `args` have a label
                    /*if !policy.is_allowed(&action) {
                        // Do not perform the action
                        continue;
                    }*/
                    let tool_result = self
                        .tools()
                        .iter()
                        .find(|&f| f == function)
                        .unwrap()
                        .call(args.clone(), datastore);
                    // TODO: Create label for this message
                    current_message = LabeledMessage {
                        message: Message::ToolResult(tool_result, id),
                        label: ProductLattice::new(Confidentiality::low(), Integrity::untrusted()),
                    }
                }
                Action::Finish(result) => return Ok(result),
            }
        }
    }
}

