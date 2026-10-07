//! Net index allocation during rendering.
use crate::error::{GeneratorError, Result};
use crate::types::ReservedNet;

/// Looks up the index of a net name while a generator renders.
pub trait NetIndexer {
    fn index(&mut self, name: &str) -> Result<u32>;
}

/// Standalone rendering: every net has index 0.
pub struct NoNets;

impl NetIndexer for NoNets {
    fn index(&mut self, _name: &str) -> Result<u32> {
        Ok(0)
    }
}

/// A fixed index for every name; terminal discovery uses it to find marker nets.
pub struct ConstantNet(pub u32);

impl NetIndexer for ConstantNet {
    fn index(&mut self, _name: &str) -> Result<u32> {
        Ok(self.0)
    }
}

/// Reserved nets keep their indices (the first entry with a name wins); new
/// names take the next free index in order of first use.
#[derive(Clone, Debug)]
pub struct NetAllocator {
    nets: Vec<ReservedNet>,
    next: u64,
}

impl NetAllocator {
    pub fn new(reserved: &[ReservedNet], next_index: u32) -> Self {
        Self {
            nets: reserved.to_vec(),
            next: u64::from(next_index),
        }
    }

    pub fn snapshot(&self) -> Vec<ReservedNet> {
        self.nets.clone()
    }
}

impl NetIndexer for NetAllocator {
    fn index(&mut self, name: &str) -> Result<u32> {
        if let Some(known) = self.nets.iter().find(|net| net.name == name) {
            return Ok(known.index);
        }
        let index = u32::try_from(self.next).map_err(|_| {
            GeneratorError::Net("Allocated net index exceeds the 32-bit range".into())
        })?;
        self.next += 1;
        self.nets.push(ReservedNet {
            name: name.to_owned(),
            index,
        });
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reserved(name: &str, index: u32) -> ReservedNet {
        ReservedNet {
            name: name.into(),
            index,
        }
    }

    #[test]
    fn allocates_in_order_of_first_use_and_reuses_names() {
        let mut nets = NetAllocator::new(&[reserved("GND", 1)], 2);
        assert_eq!(nets.index("ROW0").unwrap(), 2);
        assert_eq!(nets.index("GND").unwrap(), 1);
        assert_eq!(nets.index("COL0").unwrap(), 3);
        assert_eq!(nets.index("ROW0").unwrap(), 2);
        assert_eq!(nets.snapshot().len(), 3);
    }

    #[test]
    fn repeated_reserved_names_resolve_to_the_first_index() {
        let mut nets = NetAllocator::new(&[reserved("SHARED", 1), reserved("SHARED", 2)], 3);
        assert_eq!(nets.index("SHARED").unwrap(), 1);
        assert_eq!(nets.snapshot()[1], reserved("SHARED", 2));
    }

    #[test]
    fn the_last_32_bit_index_is_valid_and_the_next_is_not() {
        let mut nets = NetAllocator::new(&[], u32::MAX);
        assert_eq!(nets.index("ONE").unwrap(), u32::MAX);
        assert_eq!(
            nets.index("TWO").unwrap_err().message(),
            "Allocated net index exceeds the 32-bit range"
        );
    }
}
