use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::{Action, Message, Function, State, Model, Args, Arg, Datastore};
// Planning loop handle all interaction with the model, tools and users.
struct PlanningLoop<P: Plan> {
    planner: P,
    model: Model,
    tools: Vec<Function>,
}

impl<P: Plan> PlanningLoop<P> {
    // At each iteration of the loop, the current `state`, the latest `message` of the conversation
    // and the `datastore` are passed.
    fn run(&mut self, state: State, datastore: &mut Datastore, message: Message) -> String {
        let mut current_message = message;
        let mut current_state = state;
        loop {
            let action;
            (current_state, action) = self.planner.plan(current_state, current_message);
            match action {
                Action::Query(conv_history, tools) => {
                    let new_message = self.model.map(conv_history, tools);
                    current_message = new_message;
                }
                Action::MakeCall(function, args) => {
                    let tool_result = self.tools.iter().find(|&f| f == &function).unwrap().call(args, datastore);
                    current_message = tool_result;
                }
                Action::Finish(result) => return result
            }
        }
    }
}

// State passing planner which is plugged into the `PlanningLoop`
pub trait Plan {
    fn plan(&mut self, state: State, message: Message) -> (State, Action);
}

pub struct BasicPlanner {
    tools: Vec<Function>,
}

impl Plan for BasicPlanner {
    fn plan(&mut self, state: State, message: Message) -> (State, Action) {
        let mut new_state = state;
        new_state.0.push(message.clone());
        match message {
            Message::User(user_message) => {
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
            Message::Tool(tool_result) => {
                let action = Action::Query(new_state.clone(), self.tools.clone());
                (new_state, action)
            }
            Message::ToolCall(tool, args) => {
                let action = Action::MakeCall(tool, args);
                (new_state, action)
            }
            Message::Assistant(response) => {
                let action = Action::Finish(response);
                (new_state, action)
            }
        }
    }
}

pub struct VarPlanner {
    tools: Vec<Function>,
    memory: Memory,
}

pub const ID_MANAGER: AtomicUsize = AtomicUsize::new(0);

type Memory = HashMap<Variable, ToolCallResult>;
#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Variable(String);

impl Variable {
    pub fn fresh() -> Self {
        Self(format!("{}", ID_MANAGER.fetch_add(1, Ordering::Relaxed)))
    }
}

type ToolCallResult = String;

impl Plan for VarPlanner {
    fn plan(&mut self, state: State, message: Message) -> (State, Action) {
        // We need to make available variables in memory for the next tool calls
        let tools: Vec<Function> = self.tools.iter()
            .map(|tool| tool.format_vars(self.memory.keys().collect()))
            .collect();
        // This state can also be considered as the entire conversation history
        let mut new_state = state;

        match message {
            Message::User(ref user_message) => {
                new_state.0.push(message.clone());
                let action = Action::Query(new_state.clone(), tools);
                (new_state, action)
            }
            Message::Tool(tool_result) => {
                let x = Variable::fresh();
                self.memory.insert(x.clone(), tool_result);
                let var_message = Message::Tool(x.0);
                new_state.0.push(var_message);
                let action = Action::Query(new_state.clone(), tools);
                (new_state, action)
            }
            Message::ToolCall(ref tool, ref args) => {
                if tool.name() == "inspect" {
                    let tool_result = self.memory.get(&(args.0[0].clone().try_into().unwrap())).unwrap().clone();
                    new_state.0.push(Message::Tool(tool_result));
                    let action = Action::Query(new_state.clone(), tools);
                    (new_state, action)
                } else {
                    let new_args = self.expand_args(args);

                    new_state.0.push(message.clone());
                    let action = Action::MakeCall(tool.clone(), new_args);
                    (new_state, action)
                }
            }
            Message::Assistant(ref response) => {
                let action = Action::Finish(response.clone());
                new_state.0.push(message);
                (new_state, action)
            }
            _ => todo!()
        }
    }
}

impl VarPlanner {
    fn expand_args(&self, args: &Args) -> Args {
        Args(args.0.iter().map(|arg| {
            match arg {
                Arg::Basic(basic_str) => arg.clone(),
                Arg::Variable(var) => Arg::Basic(self.memory.get(&var).unwrap().clone())
            }
        })
        .collect()
        )
    }
}
