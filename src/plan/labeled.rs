use crate::{
    Action, Args, Call, Datastore, Function, Integrity, Message, Plan, PlanningLoop,
    ProductLattice, State,
    function::MetaFunction,
    ifc::{InverseLattice, Lattice, LatticeError, PowersetLattice},
    plan::{PlanError, Policy},
    tools::{EmailLabel, MetaValue},
};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, FunctionCall, Role,
};
use serde_json::{Map, Value};

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
    pub fn into_inner(self) -> Vec<MetaValue<Action, L>> {
        self.0
    }

    pub fn value(&self) -> &[MetaValue<Action, L>] {
        &self.0
    }

    pub fn value_mut(&mut self) -> &mut Vec<MetaValue<Action, L>> {
        &mut self.0
    }
}

impl<L: Lattice> Default for Trace<L> {
    fn default() -> Self {
        Self(vec![])
    }
}

pub type ActionLabel = ProductLattice<Integrity, InverseLattice<PowersetLattice<String>>>;

impl<P: Plan<State, MetaValue<Message, EmailLabel>, Action = (Action, ActionLabel)>>
    PlanningLoop<State, MetaValue<Message, EmailLabel>, MetaFunction, P>
{
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    pub async fn run_with_policy(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: MetaValue<Message, EmailLabel>,
        policy: Policy,
    ) -> Result<String, PlanError> {
        // Create a new trace of actions
        let mut trace: Trace<ActionLabel> = Trace::default();
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            let action_label;
            (current_state, (action, action_label)) = self
                .planner_mut()
                .plan(current_state, current_message.clone())
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            trace
                .value_mut()
                .push(MetaValue::new(action.clone(), action_label));

            if let Some(policy_violation) = policy.check(&trace) {
                panic!("Policy Violation {:#?}", policy_violation);
            }
            match action {
                Action::Query(conv_history, tools) => {
                    // When querying the model, this planning loop is responsible to propages the
                    // labels from the action to the model's response, signifying the inability to
                    // precisely propagate labels through LLMs.

                    // Build a chat request with all the previous conversation history and the
                    // available tools
                    let chat_request = self.model().chat(conv_history.0, tools);
                    // Send the request and save the first response choice as the new message,
                    // while also maintaining the label associated with the current loop.
                    // Note: The response from the LLM should also be checked for PII and policies
                    // associated with it.
                    current_message = MetaValue::new(
                        Message::Chat(chat_request.await?.choices[0].message.clone()),
                        current_message.label().clone(),
                    );
                }
                Action::MakeCall(ref function, ref args, id) => {
                    // Before making the actual call, we check that the call satisfies the security
                    // policy.
                    // Here both `function` and `args` have a label
                    /*if !policy.is_allowed(&action) {
                        // Do not perform the action
                        continue;
                    }*/
                    let (tool_result, label) = self
                        .tools()
                        .iter()
                        .find(|&f| f.name() == function.name())
                        .ok_or(PlanError::FunctionNotFound(function.name().to_string()))?
                        .call(args.clone(), datastore);
                    // The tool call above also issues a result and a label, which we need to
                    // convert here into a Message and a `Label`
                    let current_label = label
                        .join(current_message.label().clone())
                        .ok_or(LatticeError::LabelJoinFailed)?;
                    current_message =
                        MetaValue::new(Message::ToolResult(tool_result, id), current_label);
                }
                Action::Finish(result) => return Ok(result),
            }
        }
    }
}

pub struct TaintTrackingPlanner {
    tools: Vec<ChatCompletionTool>,
}

impl TaintTrackingPlanner {
    pub fn new(tools: Vec<ChatCompletionTool>) -> Self {
        Self { tools }
    }

    /// Normalize the arguments passed by the LLM.
    pub fn normalize_args(&self, args: String) -> Result<String, PlanError> {
        // Convert the arguments to a [`serder_json::Value`]
        let args = serde_json::from_str(&args)?;

        // If the arguments are not an object, in other words a json dictionary
        let Value::Object(map) = args else {
            // We do not support it and return an error
            return Err(PlanError::ArgumentNotObject(args));
        };

        // Create a new [`Map`] that will hold the arguments in their normalized form
        let mut new_args = Map::new();

        // For each argument
        for (arg_name, value) in map.into_iter() {
            match value {
                // If we have another map representing the argument
                Value::Object(kind_map) => {
                    // Check its kind
                    match kind_map
                        .get("kind")
                        .ok_or(PlanError::InvalidObjectKey("kind".to_string()))?
                        .as_str()
                    {
                        // If it is a value we take the value as is
                        Some("value") => new_args.insert(
                            arg_name,
                            kind_map
                                .get("value")
                                .ok_or(PlanError::InvalidObjectKey("value".to_string()))?
                                .clone(),
                        ),
                        // If it is a variable, we need to query it in the internal [`Memory`].
                        // However this is an interesting case as currently the LLM does not listen
                        // to our instructions and never returns a `kind: variable` value.
                        Some("variable") => todo!(),
                        // Any other kind value is an error
                        Some(kind) => return Err(PlanError::InvalidArgumentKind(kind.to_string())),
                        // If the kind field is missing, we return an error
                        None => return Err(PlanError::ArgumentMissingKind(arg_name)),
                    };
                }
                // If the argument schema is no a map (dict) we consider it invalid
                _ => return Err(PlanError::InvalidArgumentSchema(value)),
            }
        }

        // Convert the new map into a string and return it
        Ok(serde_json::to_string(&Value::Object(new_args))?)
    }
}

// Taint-tracking planner which is plugged into the `PlanningLoop`
impl Plan<State, MetaValue<Message, ActionLabel>> for TaintTrackingPlanner {
    type Action = (Action, ActionLabel);
    type Error = PlanError;
    // Given a [`LabeledMessage`], a security policy and a [`LabeledState`], return an action with
    // individually labeled components.
    fn plan(
        &mut self,
        state: State,
        message: MetaValue<Message, ActionLabel>,
    ) -> Result<(State, Self::Action), Self::Error> {
        // Bind the state to a mutable state such that we can update it.
        let mut new_state = state;

        // Deconstruct the `MetaValue` such that we get individual access to the message and the
        // label passed
        let (message, label) = message.into_raw_parts();

        // Create a new state, action and action label based on the message that we get. This match
        // also converts the message from a completion response type message to a completion
        // request type message.
        let (new_state, action) = match message {
            // If we have a chat message between the user and the assistant.
            Message::Chat(message) => {
                // Get the role of the message
                let role = message.role;
                // Convert the message and create a new action depending on the role
                match role {
                    Role::User => {
                        // For user messages we only care about the content
                        let conv_message = ChatCompletionRequestUserMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoUserContent)?)
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);
                        // In this case, the action to take is to query the LLM with the updated
                        // state and the set of available tools
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Tool => {
                        // For tools messages we want to capture the content of the tool aka the
                        // result that the tool sent back and the tool's id, such that the LLM
                        // can match the tool call with the tool result.
                        let conv_message = ChatCompletionRequestToolMessageArgs::default()
                            .content(message.content.ok_or(PlanError::NoToolContent)?)
                            .tool_call_id(
                                message.tool_calls.ok_or(PlanError::NoToolCalls)?[0]
                                    .id
                                    .clone(),
                            )
                            .build()?
                            .into();
                        // Update the state with the new message
                        new_state.0.push(conv_message);

                        // In this case, the action to take is to query the LLM with the updated
                        // state and the set of available tools
                        let action = Action::Query(new_state.clone(), self.tools.clone());
                        (new_state, action)
                    }
                    Role::Assistant => {
                        // If we have an assistant message, our response depends on whether the
                        // message is a tool call or a pure chat message.

                        // In the case of a tool call.
                        if let Some(tool_calls) = message.tool_calls {
                            // Currently there is no support for multiple tool calls in one
                            // message.
                            assert!(tool_calls.len() == 1);
                            // Get the name and argument of the first tool call.
                            let FunctionCall { name, arguments } = tool_calls[0].clone().function;

                            // Normalize arguments such that we could parse them in their correct
                            // function input
                            let arguments = self.normalize_args(arguments);

                            // Convert the message to a request to update the state
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .tool_calls(vec![tool_calls[0].clone()])
                                .build()?
                                .into();
                            // Update the state with the new message
                            new_state.0.push(conv_message);

                            // In this case, the action to take is to call the specified tool with
                            // the specified arguments, keeping the id of the tool call such that
                            // we can report it back to the LLM in the message that will contain
                            // the tool result.
                            let action = Action::MakeCall(
                                Function::new(name),
                                Args(arguments?),
                                tool_calls[0].clone().id,
                            );
                            (new_state, action)
                        // In the case of an assitant pure chat message
                        } else if let Some(content) = message.content {
                            // Convert the message response into a request and copy over the
                            // contents of the message
                            let conv_message = ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into();
                            // Update the state with the new message
                            new_state.0.push(conv_message);
                            // In this case, the assistant gave the "final" answer as we want to
                            // take a finishing action and return the result to the caller.
                            let action = Action::Finish(content);
                            (new_state, action)
                        } else {
                            todo!();
                        }
                    }
                    _ => unimplemented!(),
                }
            }
            // If we have a tool result, we are in a similar case with the chat message in the tool
            // role above. However this is separate since this type of message is generated by the
            // current process and not by the LLM in order to fill it with a tool result.
            Message::ToolResult(content, id) => {
                // Convert the message to a request to update the state
                let conv_message = ChatCompletionRequestToolMessageArgs::default()
                    .content(content)
                    .tool_call_id(id)
                    .build()?
                    .into();
                // Update the state with the new message
                new_state.0.push(conv_message);

                // In this case, the action to take is to query the LLM with the updated
                // state and the set of available tools
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
        };
        Ok((new_state, (action, label)))
    }
}
