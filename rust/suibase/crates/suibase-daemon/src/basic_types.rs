// Some common types depending only on built-in or "standard" types.
use std::sync::atomic::{AtomicUsize, Ordering};
pub type EpochTimestamp = tokio::time::Instant;
pub type IPKey = String;

pub type InstanceID = usize;
pub fn gen_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// An fix sized array with management of used/empty cells.
pub type ManagedVecUSize = u8;
pub struct ManagedVec<T> {
    data: Vec<Option<T>>,
    some_len: ManagedVecUSize,
}

// The elements must have the trait ManagedVecElement.
// Needed?
trait ManagedVecElement {
    fn set_idx(&self);
    fn get_idx(&self) -> ManagedVecUSize;
}

impl<T> ManagedVec<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            some_len: 0,
        }
    }

    // That is the only time the index is returned.
    pub fn push(&mut self, value: T) -> Option<ManagedVecUSize> {
        self.some_len += 1;
        // Iterate to find a free cell before creating a new one.
        for (index, cell) in self.data.iter_mut().enumerate() {
            if cell.is_none() {
                *cell = Some(value);
                return Some(index.try_into().unwrap());
            }
        }
        let index = self.data.len();
        self.data.push(Some(value));
        Some(index.try_into().unwrap())
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
        self.data.get_mut(usize_index).and_then(|v| v.take())
    }

    pub fn len(&self) -> ManagedVecUSize {
        self.some_len
    }
}
