use crate::{
    Action, Arg, Args, Confidentiality, ConversationHistory, Datastore, Function, Integrity, Label,
    LabeledMessage, Message, Plan, PlanningLoop, ProductLattice, plan::PlanError, tools::Memory,
    Call
};
use async_openai::types::ChatCompletionRequestMessage;

#[derive(Clone)]
pub struct LabeledConversationHistory<M> {
    _conv: ConversationHistory<M>,
    _label: Label,
}

type LabeledState = LabeledConversationHistory<ChatCompletionRequestMessage>;

pub enum LabeledAction {
    // Query the model with a specific conversation history and available tools
    Query(LabeledState, Vec<LabeledFunction>),
    // Call a `Tool` with `Args`
    MakeCall(LabeledFunction, LabeledArgs, String),
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

impl Call for LabeledFunction {
    type Args = LabeledArgs;
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> String {
        todo!()
    }
}

pub struct Policy;

impl Policy {
    fn _is_allowed(&self, _action: &LabeledAction) -> bool {
        true
    }
}

impl LabeledFunction {
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    pub fn _call(&self, args: LabeledArgs, datastore: &mut Datastore) -> String {
        self.function.call(
            Args(args.0.iter().map(|x| x.arg.to_string()).collect()),
            datastore,
        )
    }
}

impl<P: Plan<LabeledState, LabeledMessage, Action=LabeledAction>> PlanningLoop<LabeledState, LabeledMessage, LabeledFunction, P> {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub fn run_with_policy(
        &mut self,
        state: LabeledState,
        datastore: &mut Datastore,
        message: LabeledMessage,
        _policy: Policy,
    ) -> Result<String, PlanError> {
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self
                .planner_mut()
                .plan(current_state, current_message.clone())
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            match action {
                LabeledAction::Query(_conv_history, _tools) => {
                    todo!()
                }
                LabeledAction::MakeCall(ref function, ref args, id) => {
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
                LabeledAction::Finish(result) => return Ok(result),
            }
        }
    }
}

pub struct TaintTrackingPlanner {
    tools: Vec<Function>,
    memory: Memory,
    policy: Policy,
}

// Taint-tracking planner which is plugged into the `PlanningLoop`
impl Plan<LabeledState, LabeledMessage> for TaintTrackingPlanner {
    type Action = LabeledAction;
    type Error = PlanError;
    // Given a [`LabeledMessage`], a security policy and a [`LabeledState`], return an action with
    // individually labeled components.
    fn plan(&mut self, state: LabeledState, message: LabeledMessage) -> Result<(LabeledState, Self::Action), Self::Error> {
        todo!()
    }
}
