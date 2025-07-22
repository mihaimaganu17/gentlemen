use std::{cmp::Ordering, collections::HashSet, hash::Hash};

pub trait Lattice: PartialOrd + Sized + Clone {
    /// Returns the least upper bound between `self` and `other` values
    fn join(self, other: Self) -> Option<Self>;
    /// Returns the greatest lower bound between `self` and `other` values
    fn meet(self, other: Self) -> Option<Self>;
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Confidentiality {
    // Public information
    Low = 0,
    // Secret information
    High = 1,
}

impl Lattice for Confidentiality {
    fn join(self, other: Self) -> Option<Self> {
        Some(if self <= other { other } else { self })
    }

    fn meet(self, other: Self) -> Option<Self> {
        Some(if self <= other { self } else { other })
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

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Integrity {
    // Low integrity
    Untrusted = 0,
    // High integrity
    Trusted = 1,
}

impl Lattice for Integrity {
    fn join(self, other: Self) -> Option<Self> {
        Some(if self <= other { other } else { self })
    }

    fn meet(self, other: Self) -> Option<Self> {
        Some(if self <= other { self } else { other })
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

// Information lattice corresponding to the product of 2 other lattices
#[derive(Debug, PartialEq, Clone)]
pub struct ProductLattice<A: Lattice, B: Lattice> {
    lattice1: A,
    lattice2: B,
}

impl<A: Lattice, B: Lattice> PartialOrd for ProductLattice<A, B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ord1 = self.lattice1.partial_cmp(&other.lattice1)?;
        let ord2 = self.lattice2.partial_cmp(&other.lattice2)?;
        if ord1 == ord2 {
            // If the 2 are equal, we return the result
            Some(ord1)
        // If at least one is smaller than the other and the other is equal, we return `Less`
        } else if ord1 == Ordering::Less && ord2 == Ordering::Equal
            || ord1 == Ordering::Equal && ord2 == Ordering::Less
        {
            Some(Ordering::Less)
        // Otherwise greater
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl<A: Lattice, B: Lattice> Lattice for ProductLattice<A, B> {
    /// Returns the least upper bound between `self` and `other` values
    fn join(self, other: Self) -> Option<Self> {
        let lattice1 = self.lattice1.join(other.lattice1)?;
        let lattice2 = self.lattice2.join(other.lattice2)?;

        Some(Self { lattice1, lattice2 })
    }

    /// Returns the greatest lower bound between `self` and `other` values
    fn meet(self, other: Self) -> Option<Self> {
        let lattice1 = self.lattice1.meet(other.lattice1)?;
        let lattice2 = self.lattice2.meet(other.lattice2)?;

        Some(Self { lattice1, lattice2 })
    }
}

impl<A: Lattice, B: Lattice> ProductLattice<A, B> {
    pub fn new(lattice1: A, lattice2: B) -> Self {
        Self { lattice1, lattice2 }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PowersetLattice<T: Eq + Hash> {
    subset: HashSet<T>,
    universe: HashSet<T>,
}

impl<T: Eq + Hash> PowersetLattice<T> {
    pub fn new(subset: HashSet<T>, universe: HashSet<T>) -> Result<Self, LatticeError> {
        if !subset.is_subset(&universe) {
            return Err(LatticeError::SubsetNotInUniverse);
        }

        Ok(Self { subset, universe })
    }
}

impl<T: Eq + Hash> PartialOrd for PowersetLattice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.subset == other.subset {
            Some(Ordering::Equal)
        } else if self.subset.is_subset(&other.subset) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl<T: Eq + Hash + Clone> Lattice for PowersetLattice<T> {
    /// Returns the least upper bound between `self` and `other` values
    fn join(self, other: Self) -> Option<Self> {
        // Union of the 2 subsets
        let subset = &self.subset | &other.subset;

        Self::new(subset, self.universe).ok()
    }

    /// Returns the greatest lower bound between `self` and `other` values
    fn meet(self, other: Self) -> Option<Self> {
        // Intersection of the 2 subsets
        let subset = &self.subset & &other.subset;

        Self::new(subset, self.universe).ok()
    }
}

// Information lattice which inverses the order of operations
#[derive(Debug, PartialEq, Clone)]
pub struct InverseLattice<T: Lattice> {
    inner: T,
}

impl<T: Lattice> InverseLattice<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Lattice> PartialOrd for InverseLattice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}

impl<T: Lattice> Lattice for InverseLattice<T> {
    fn join(self, other: Self) -> Option<Self> {
        Some(Self::new(self.inner.meet(other.inner)?))
    }

    fn meet(self, other: Self) -> Option<Self> {
        Some(Self::new(self.inner.join(other.inner)?))
    }
}

#[derive(Debug)]
pub enum LatticeError {
    SubsetNotInUniverse,
}

pub type Label = ProductLattice<Confidentiality, Integrity>;
