use crate::{
    Call, Confidentiality, Datastore, Integrity, LabeledArgs, LabeledFunction, LabeledMessage,
    LabeledState, Message, Plan, PlanningLoop, ProductLattice, plan::PlanError, tools::Memory,
};
use async_openai::types::ChatCompletionTool;
use std::collections::HashMap;

pub enum LabeledAction {
    // Query the model with a specific conversation history and available tools
    Query(LabeledState, Vec<LabeledFunction>),
    // Call a `Tool` with `Args`
    MakeCall(LabeledFunction, LabeledArgs, String),
    // Finish the conversation and respond to the user.
    Finish(String),
}

pub struct Policy;

impl Policy {
    fn _is_allowed(&self, _action: &LabeledAction) -> bool {
        true
    }
}

impl<P: Plan<LabeledState, LabeledMessage, Action = LabeledAction>>
    PlanningLoop<LabeledState, LabeledMessage, LabeledFunction, P>
{
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub async fn run_with_policy(
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
                    // When querying the model, this planning loop is responsible to propages the
                    // labels from the action to the model's response, signifying the inability to
                    // precisely propagate labels through LLMs.
                    todo!()
                }
                LabeledAction::MakeCall(ref function, ref args, id) => {
                    // Before making the actual call, we check that the call satisfies the security
                    // policy.
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
                    current_message = LabeledMessage::new(
                        Message::ToolResult(tool_result, id),
                        ProductLattice::new(Confidentiality::low(), Integrity::untrusted()),
                    )
                }
                LabeledAction::Finish(result) => return Ok(result),
            }
        }
    }
}

pub struct TaintTrackingPlanner {
    tools: Vec<ChatCompletionTool>,
    memory: Memory,
    policy: Policy,
}

impl TaintTrackingPlanner {
    pub fn new(tools: Vec<ChatCompletionTool>, policy: Policy) -> Self {
        Self {
            tools,
            memory: HashMap::new(),
            policy,
        }
    }
}

// Taint-tracking planner which is plugged into the `PlanningLoop`
impl Plan<LabeledState, LabeledMessage> for TaintTrackingPlanner {
    type Action = LabeledAction;
    type Error = PlanError;
    // Given a [`LabeledMessage`], a security policy and a [`LabeledState`], return an action with
    // individually labeled components.
    fn plan(
        &mut self,
        _state: LabeledState,
        _message: LabeledMessage,
    ) -> Result<(LabeledState, Self::Action), Self::Error> {
        let email_universe = crate::tools::EmailAddressUniverse::new(&crate::tools::INBOX);
        println!("{:#?}", email_universe);
        todo!()
    }
}
