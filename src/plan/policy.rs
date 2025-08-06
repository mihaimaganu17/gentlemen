use super::labeled::{Trace, ActionLabel};
use crate::{Integrity, Action, tools::SendSlackMessageArgs, ifc::Lattice};

pub fn contains_url(text: &str) -> Result<bool, regex::Error> {
    let pattern =
        r"http[s]?://
        (?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\\(\\),]|
        (?:%[0-9a-fA-F][0-9a-fA-F]))+"; // communication protocol + domain + port

    let re = regex::Regex::new(pattern)?;
    Ok(re.is_match(text))
}

/// Policy that stops sending untrusted Teams messages containing a URL.
pub fn policy_no_untrusted_url(trace: Trace<ActionLabel>) -> Option<PolicyViolation> {
    if let (Action::MakeCall(function, args, id), label) = trace.value().last()?.raw_parts() {
        if function.name().starts_with("send_teams_message") {
            println!("Checking tool call {:?} -> {:?}({:?}) with label {:?}", id, function, args, label);
            let args: SendSlackMessageArgs = serde_json::from_str(&args.0).ok()?;
            // Check if the integrity label of the message is `untrusted` and if the message
            // contains an URL.
            if label.lattice1() == &Integrity::Untrusted && contains_url(args.message()).ok()? {
                Some(PolicyViolation::Standard("Attempted to send a message with an untrusted URL".to_string()))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

#[derive(Debug)]
pub enum PolicyViolation {
    Standard(String),
}
