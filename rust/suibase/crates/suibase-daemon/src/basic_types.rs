// Some common types depending only on built-in or "standard" types.
pub type EpochTimestamp = tokio::time::Instant;

/*
use std::sync::atomic::{AtomicUsize, Ordering};

pub type InstanceID = usize;
pub fn gen_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}*/

// Some duration are stored in micro-seconds. In many context,
// above 1 min is a likely bug (with the benefit that the limit
// can be stored into 32-bits without failure).
pub const MICROSECOND_LIMIT: u32 = 60000000; // 1 minute
pub fn duration_to_micros(value: std::time::Duration) -> u32 {
    match value.as_micros().try_into() {
        Ok(value) => std::cmp::max(value, MICROSECOND_LIMIT),
        Err(_) => MICROSECOND_LIMIT,
    }
}

pub type InputPortIdx = ManagedVecUSize;
pub type TargetServerIdx = ManagedVecUSize;
pub type WorkdirIdx = ManagedVecUSize;

// A fix sized array with recycling of empty cells.
//
// This is used for fast indexing tricks versus HashMap lookup.
//
// Optimized for relatively small arrays that rarely changes and owns
// its elements.
//
// Intended use case: configuration in memory that must be lookup often
// in a RwLock.
//
// --------
//
// Stored elements should have a variable like this:
//
//   struct MyStruct {
//      managed_idx: Option<ManagedVecUSize>, ...
//   }
//   impl MyStruct {
//      fn new() -> Self { managed_idx: None, ... }
//   }
//
// and implement the ManagedElement Trait.
//
// The managed_idx should be initialized only by the ManagedVec.
//
// This "managed_idx" can be copied in other data structure (like a "pointer")
// and be later used with get() and get_mut() for fast access.

pub type ManagedVecUSize = u8;
pub struct ManagedVec<T> {
    data: Vec<Option<T>>,
    some_len: ManagedVecUSize,
}

pub trait ManagedElement {
    fn managed_idx(&self) -> Option<ManagedVecUSize>;
    fn set_managed_idx(&mut self, index: Option<ManagedVecUSize>);
}

impl<T: ManagedElement> ManagedVec<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            some_len: 0,
        }
    }

    // That is the only time the index is set and returned.
    pub fn push(&mut self, mut value: T) -> Option<ManagedVecUSize> {
        self.some_len += 1;
        // Iterate to find a free cell before creating a new one.
        for (index, cell) in self.data.iter_mut().enumerate() {
            if cell.is_none() {
                let managed_idx: ManagedVecUSize = index.try_into().unwrap();
                value.set_managed_idx(Some(managed_idx));
                *cell = Some(value);
                return Some(managed_idx);
            }
        }

        let index = self.data.len();
        let managed_idx: ManagedVecUSize = index.try_into().unwrap();
        self.data.push(Some(value));
        Some(managed_idx)
    }

    pub fn get(&self, index: ManagedVecUSize) -> Option<&T> {
        let usize_index = usize::from(index);
        self.data.get(usize_index).and_then(|v| v.as_ref())
    }

    pub fn get_mut(&mut self, index: ManagedVecUSize) -> Option<&mut T> {
        self.data
            .get_mut(usize::from(index))
            .and_then(|v| v.as_mut())
    }

    // This free the cells for re-use. If a push is done, it
    // might re-use that cell (the same index).
    pub fn remove(&mut self, index: ManagedVecUSize) -> Option<T> {
        let usize_index = usize::from(index);
        if self.data.get(usize_index).is_none() {
            return None;
        }
        self.some_len -= 1;
        let ret_value = self.data.get_mut(usize_index).and_then(|v| v.take());
        // Shrink the vector by removing all empty last cells.
        while let Some(None) = self.data.last() {
            self.data.pop();
        }
        ret_value
    }

    pub fn len(&self) -> ManagedVecUSize {
        self.some_len
    }

    // Implement Iter and IterMut to iterate over the used cells.
    pub fn into_iter(self) -> impl Iterator<Item = (ManagedVecUSize, T)> {
        self.data
            .into_iter()
            .enumerate()
            .filter_map(|(index, cell)| cell.map(|value| (index.try_into().unwrap(), value)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ManagedVecUSize, &T)> {
        self.data.iter().enumerate().filter_map(|(index, cell)| {
            cell.as_ref()
                .map(|value| (index.try_into().unwrap(), value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ManagedVecUSize, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .filter_map(|(index, cell)| {
                cell.as_mut()
                    .map(|value| (index.try_into().unwrap(), value))
            })
    }
}

#[test]

fn len() {
    struct TS {
        managed_idx: Option<ManagedVecUSize>,
        _value: u8,
    }

    impl TS {
        pub fn new(_value: u8) -> Self {
            Self {
                managed_idx: None,
                _value,
            }
        }
    }

    impl ManagedElement for TS {
        fn managed_idx(&self) -> Option<ManagedVecUSize> {
            self.managed_idx
        }
        fn set_managed_idx(&mut self, index: Option<ManagedVecUSize>) {
            self.managed_idx = index;
        }
    }

    let mut v1 = ManagedVec::<TS>::new();
    assert_eq!(v1.len(), 0);
    v1.push(TS::new(1));
    assert_eq!(v1.len(), 1);
    v1.push(TS::new(2));
    assert_eq!(v1.len(), 2);
    v1.remove(0);
    assert_eq!(v1.len(), 1);
    v1.remove(0);
    assert_eq!(v1.len(), 0);
    v1.push(TS::new(1));
    v1.push(TS::new(2));
    v1.push(TS::new(3));
    assert_eq!(v1.len(), 3);
    v1.remove(1);
    assert_eq!(v1.len(), 2);
    v1.remove(1);
    assert_eq!(v1.len(), 1);
    v1.push(TS::new(2));
    assert_eq!(v1.len(), 2);
}
