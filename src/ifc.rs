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

#[derive(Debug, PartialEq)]
pub struct Product<A: Lattice, B: Lattice> {
    lattice1: A,
    lattice2: B,
}

impl<A: Lattice, B: Lattice> PartialOrd for Product<A, B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ord1 = self.lattice1.partial_cmp(&other.lattice1)?;
        let ord2 = self.lattice2.partial_cmp(&other.lattice2)?;
        if ord1 == ord2 {
            // If the 2 are equal, we return the result
            Some(ord1)
        } else {
            if ord1 == Ordering::Less && ord2 == Ordering::Equal
                || ord1 == Ordering::Equal && ord2 == Ordering::Less {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Greater)
            }
        }
    }
}

impl<A: Lattice, B: Lattice> Lattice for Product<A, B> {
    /// Returns the least upper bound between `self` and `other` values
    fn join(self, other: Self) -> Self {
        let lattice1 = self.lattice1.join(other.lattice1);
        let lattice2 = self.lattice2.join(other.lattice2);

        Self {
            lattice1,
            lattice2,
        }
    }

    /// Returns the greatest lower bound between `self` and `other` values
    fn meet(self, other: Self) -> Self {
        let lattice1 = self.lattice1.meet(other.lattice1);
        let lattice2 = self.lattice2.meet(other.lattice2);

        Self {
            lattice1,
            lattice2,
        }
    }
}

