use crate::{
    Action, Call, Confidentiality, Datastore, Function, Integrity, Message, Plan, PlanningLoop,
    ProductLattice, State,
    ifc::{Lattice, InverseLattice, PowersetLattice},
    plan::PlanError,
    tools::{Memory, MetaValue},
};
use async_openai::types::ChatCompletionTool;
use std::collections::HashMap;

pub struct Policy;

impl Policy {
    fn _is_allowed(&self, _action: &Action) -> bool {
        true
    }
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
// When non-empty, such a label applied to all fields of that node and below.
//
// Also attach metadata field to label individual messages in the conversation history.
// The initial system and user messages are typically considered trusted and public and by default.

// A trace is a sequence of actions that the model takes starting from a user's Message::Query
// and ending with an `Action::Finish`.
pub struct Trace<L: Lattice>(Vec<MetaValue<Action, L>>);

impl<L: Lattice> Trace<L> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn into_inner(self) -> Vec<MetaValue<Action, L>> {
        self.0
    }

    pub fn value(&self) -> &[MetaValue<Action, L>] {
        &self.0
    }
}

pub type ActionLabel<'a> = ProductLattice<Integrity, InverseLattice<PowersetLattice<&'a str>>>;

impl<P: Plan<State, Message, Action = Action>> PlanningLoop<State, Message, Function, P> {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub async fn run_with_policy<L: Lattice>(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: Message,
        _label: L,
        _policy: Policy,
    ) -> Result<String, PlanError> {
        // Create a new trace of actions
        let mut _trace: Trace<ActionLabel<'_>> = Trace::new();
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self
                .planner_mut()
                .plan(current_state, current_message.clone())
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            match action {
                Action::Query(_conv_history, _tools) => {
                    // When querying the model, this planning loop is responsible to propages the
                    // labels from the action to the model's response, signifying the inability to
                    // precisely propagate labels through LLMs.
                    todo!()
                }
                Action::MakeCall(ref function, ref args, id) => {
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
                    // The tool call above also issues a result and a label, which we need to
                    // convert here into a Message and a `Label`
                    let _label = ProductLattice::new(Confidentiality::low(), Integrity::untrusted());
                    current_message = Message::ToolResult(tool_result, id);
                }
                Action::Finish(result) => return Ok(result),
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
impl Plan<State, Message> for TaintTrackingPlanner {
    type Action = Action;
    type Error = PlanError;
    // Given a [`LabeledMessage`], a security policy and a [`LabeledState`], return an action with
    // individually labeled components.
    fn plan(
        &mut self,
        _state: State,
        _message: Message,
    ) -> Result<(State, Self::Action), Self::Error> {
        let _email_universe = crate::tools::EmailAddressUniverse::new(&crate::tools::INBOX);
        todo!()
    }
}
