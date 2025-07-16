use super::{Plan, PlanError};
use crate::{Action, Datastore, Function, Message, State, openai::LlmClient, Call};
use std::marker::PhantomData;

/// Planning loop orchestrates the communication with the model and handles the `Planner`'s
/// required actions.
pub struct PlanningLoop<S, M: Clone, F: Call, P: Plan<S, M>> {
    // The planner used to plan the next action in the loop
    planner: P,
    // The LLM model used to accomplish the task
    model: LlmClient,
    // The tools the LLM model has access to
    tools: Vec<F>,
    // Phantom data such that we can bind the type of `Message` that the planner `P` uses
    phantom_message: PhantomData<M>,
    phantom_state: PhantomData<S>,
}

impl<S, M: Clone, F: Call, P: Plan<S, M>> PlanningLoop<S, M, F, P> {
    pub fn planner_mut(&mut self) -> &mut P {
        &mut self.planner
    }

    pub fn tools(&mut self) -> &[F] {
        self.tools.as_ref()
    }

    /// Create a new `PlanninLoop` with an action `planner` a `model` to do the work and available
    /// `tools` that the model can call
    pub fn new(planner: P, model: LlmClient, tools: Vec<F>) -> Self {
        Self {
            planner,
            model,
            tools,
            phantom_message: PhantomData,
            phantom_state: PhantomData,
        }
    }
}

impl<P: Plan<State, Message, Action=Action>> PlanningLoop<State, Message, Function, P> {
    /// The entry point for executing the `PlanningLoop`. At each iteration of the loop, the
    /// current `state`, the latest `message` of the conversation and the `datastore` are passed.
    pub async fn run(
        &mut self,
        state: State,
        datastore: &mut Datastore,
        message: Message,
    ) -> Result<String, PlanError> {
        // Bind the given message to a mutable variable as it will be updated inside the following
        // loop based on what action the loop is taking.
        let mut current_message = message;
        // Bind the given state to a mutable variable as it will be updates insied the following
        // loop with a new message.
        let mut current_state = state;
        loop {
            let action;
            // Plan the next action giving the current message and state. The new message is sent
            // separate from the state as it will be converted by the planner from a
            // `ChatCompletionRequest{Type}` message to a `ChatCompletionResponse{Type}` message.
            (current_state, action) = self
                .planner
                .plan(current_state, current_message)
                .map_err(|e| PlanError::CannotPlan(format!("{:?}", e)))?;
            match action {
                // We have to query the model
                Action::Query(conv_history, tools) => {
                    // Build a chat request with all the previous conversation history and the
                    // available tools
                    let chat_request = self.model.chat(conv_history.0, tools);
                    // Send the request and save the first response choice as the new message
                    current_message = Message::Chat(chat_request.await?.choices[0].message.clone());
                }
                // We have to call a tool requested by the model
                Action::MakeCall(function, args, id) => {
                    // Find the requested `function` and call it with the given arguments and using
                    // the available datastore.
                    let tool_result = self
                        .tools
                        .iter()
                        .find(|&f| f == &function)
                        .unwrap()
                        .call(args, datastore);
                    // New message represents the result we got from calling the above tool and we
                    // also keep the tool id such that the model can associate the tools request
                    // with the tool id.
                    current_message = Message::ToolResult(tool_result, id);
                }
                // We got the final model response and we return it back to the caller
                Action::Finish(result) => return Ok(result),
            }
        }
    }
}
