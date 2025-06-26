use std::cmp::Ordering;

pub trait Lattice: PartialOrd {
    /// Returns the least upper bound between `self` and `other` values
    fn join(self, other: Self) -> Self;
    /// Returns the greatest lower bound between `self` and `other` values
    fn meet(self, other: Self) -> Self;
}

#[derive(Debug, PartialEq)]
pub enum Confidentiality {
    // Public information
    Low = 0,
    // Secret information
    High = 1,
}

impl PartialOrd for Confidentiality {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.partial_cmp(other)
    }
}

impl Lattice for Confidentiality {
    fn join(self, other: Self) -> Self {
        if self <= other {
            other
        } else {
            self
        }
    }

    fn meet(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }
}

impl Confidentiality {
    pub fn low() -> Self {
        Self::Low
    }

    pub fn high() -> Self {
        Self::High
    }
}

#[derive(Debug, PartialEq)]
pub enum Integrity {
    // Low integrity
    Untrusted = 0,
    // High integrity
    Trusted = 1,
}

impl PartialOrd for Integrity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.partial_cmp(other)
    }
}

impl Lattice for Integrity {
    fn join(self, other: Self) -> Self {
        if self <= other {
            other
        } else {
            self
        }
    }

    fn meet(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }
}

impl Integrity {
    pub fn trusted() -> Self {
        Self::Trusted
    }

    pub fn untrusted() -> Self {
        Self::Untrusted
    }
}

