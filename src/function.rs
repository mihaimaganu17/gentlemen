use crate::tools::{ReadEmailsArgs, SendSlackMessageArgs, read_emails, send_slack_message};
use crate::{Datastore, Label};
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Function(String);

impl Function {
    pub fn new(inner: String) -> Self {
        Self(inner)
    }
}

pub trait Call {
    type Args;
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> String;
}

impl Call for Function {
    type Args = Args;
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> String {
        match self.0.as_str() {
            "read_emails" => {
                // Convert args to desired type
                let args: ReadEmailsArgs = serde_json::from_str(&args.0).unwrap();
                let result = read_emails(args);
                println!("{result:?}");
                serde_json::to_string(&result).unwrap()
            }
            "send_slack_message" => {
                let args: SendSlackMessageArgs = serde_json::from_str(&args.0).unwrap();
                let result = send_slack_message(args);
                println!("{result:?}");
                serde_json::to_string(&result).unwrap()
            }
            "read_emails_labeled" => {
                // Convert args to desired type
                let args: ReadEmailsArgs = serde_json::from_str(&args.0).unwrap();
                let result = crate::tools::read_emails_labeled(args, &crate::tools::INBOX);
                serde_json::to_string(&result).unwrap()
            }
            _ => panic!("{:?}", self.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Args(pub String);

#[derive(Clone)]
pub enum Arg {
    Basic(String),
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic(value) => write!(f, "{}", value),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct LabeledFunction {
    name: String,
    label: Label,
}

impl LabeledFunction {
    pub fn new(name: String, label: Label) -> Self {
        Self { name, label }
    }

    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    pub fn _call(&self, args: LabeledArgs, datastore: &mut Datastore) -> String {
        Function::new(self.name.clone()).call(
            Args(args.0.iter().map(|x| x.arg.to_string()).collect()),
            datastore,
        )
    }
}

#[derive(Clone)]
pub struct LabeledArgs(Vec<LabeledArg>);

#[derive(Clone)]
pub struct LabeledArg {
    arg: Arg,
    _label: Label,
}

impl Call for LabeledFunction {
    type Args = LabeledArgs;
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, _args: Self::Args, _datastore: &mut Datastore) -> String {
        todo!()
    }
}

#[derive(Debug)]
pub enum ConversionError {
    ArgIsNotVariable,
}
