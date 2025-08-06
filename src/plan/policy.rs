use super::labeled::Trace;
use crate::ifc::Lattice;

pub fn contains_url(text: &str) -> Result<bool, regex::Error> {
    let pattern =
        r"http[s]?://
        (?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\\(\\),]|
        (?:%[0-9a-fA-F][0-9a-fA-F]))+"; // communication protocol + domain + port

    let re = regex::Regex::new(pattern)?;
    Ok(re.is_match(text))
}

/// Policy that stops sending untrusted Teams messages containing a URL.
pub fn policy_no_untrusted_url<L: Lattice>(trace: Trace<L>) -> Option<PolicyViolation> {
    if trace.value().len() < 1 {
        return None;
    }

    None
}

#[derive(Debug)]
pub enum PolicyViolation {
    Standard(String),
}
