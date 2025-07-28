use crate::tools::{ReadEmailsArgs, SendSlackMessageArgs, read_emails, send_slack_message, MetaValue, EmailLabel};
use crate::{Datastore, Label};
use crate::ifc::Lattice;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Function(String);

impl Function {
    pub fn new(inner: String) -> Self {
        Self(inner)
    }

    pub fn name(&self) -> &str {
        self.0.as_ref()
    }
}

pub trait Call {
    type Args;
    type Output;
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> Self::Output;
}

impl Call for Function {
    type Args = Args;
    type Output = String;
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> Self::Output {
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
            _ => panic!("{:?}", self.0),
        }
    }
}

/// Similar with `Function` but we return the result of the function call along with the `Label` of
/// the result
#[derive(Debug, PartialEq, Clone)]
pub struct MetaFunction<'a>{
    name: &'a str,
}

impl<'a> Call for MetaFunction<'a> {
    type Args = Args;
    type Output = (String, EmailLabel<'a>);
    // A function reads from and writes to a global datastore. This allows for interaction between
    // tools and capture side effects through update to the datastore.
    // Currently in this model we return an updated datastore.
    fn call(&self, args: Self::Args, _datastore: &mut Datastore) -> Self::Output {
        match self.name {
            "read_emails_labeled" => {
                // Convert args to desired type
                let args: ReadEmailsArgs = serde_json::from_str(&args.0).unwrap();
                let MetaValue { value, label } = crate::tools::read_emails_labeled(args, &crate::tools::INBOX).into_inner();
                let value = value.into_iter().map(|mv| format!("{:?}", mv.value())).collect::<Vec<_>>();
                (serde_json::to_string(&value).unwrap(), label)
            }
            _ => todo!()
        }
    }
}

impl<'a> MetaFunction<'a> {
    pub fn name(&self) -> &'a str {
        self.name
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

#[derive(Debug)]
pub enum ConversionError {
    ArgIsNotVariable,
}
