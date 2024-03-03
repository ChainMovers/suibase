// A fix sized array with recycling of empty cells.
//
// This is used for fast indexing tricks versus HashMap lookup.
//
// Optimized for relatively small arrays (,256 elements) that
// rarely changes.
//
// Intended use case: configuration in memory that must be lookup often
// in a RwLock.
//
// --------
//
// Stored elements should have a variable like this:
//
//   struct MyStruct {
//      idx: Option<ManagedVecU8>, ...
//   }
//   impl MyStruct {
//      fn new() -> Self { idx: None, ... }
//   }
//
// and implement the ManagedElement Trait.
//
// The 'idx' should be initialized only by the ManagedVec.
//
// This 'idx' can be copied in other data structure (like a "pointer")
// and be later used with get() and get_mut() for fast access.

pub type ManagedVecU8 = u8;

#[derive(Debug)]
pub struct ManagedVec<T> {
    data: Vec<Option<T>>,
    some_len: ManagedVecU8,
}

pub trait ManagedElement {
    fn idx(&self) -> Option<ManagedVecU8>;
    fn set_idx(&mut self, index: Option<ManagedVecU8>);
}

impl<T: ManagedElement> ManagedVec<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            some_len: 0,
        }
    }

    // That is the only time the index is set and returned.
    // TODO Verify handling of out of range index.
    pub fn push(&mut self, mut value: T) -> Option<ManagedVecU8> {
        self.some_len += 1;
        // Iterate to find a free cell before creating a new one.
        for (index, cell) in self.data.iter_mut().enumerate() {
            if cell.is_none() {
                let managed_idx: ManagedVecU8 = index.try_into().unwrap();
                value.set_idx(Some(managed_idx));
                *cell = Some(value);
                return Some(managed_idx);
            }
        }

        let index = self.data.len();
        let managed_idx: ManagedVecU8 = index.try_into().unwrap();
        value.set_idx(Some(managed_idx));
        self.data.push(Some(value));
        Some(managed_idx)
    }

    // TODO Verify getting OOB have no effect.
    pub fn get(&self, index: ManagedVecU8) -> Option<&T> {
        let usize_index = usize::from(index);
        self.data.get(usize_index).and_then(|v| v.as_ref())
    }

    // TODO Verify getting OOB have no effect.
    pub fn get_mut(&mut self, index: ManagedVecU8) -> Option<&mut T> {
        self.data
            .get_mut(usize::from(index))
            .and_then(|v| v.as_mut())
    }

    // This free the cells for re-use. If a push is done, it
    // might re-use that cell (the same index).
    //
    // TODO Verify remove OOB have no effect.
    pub fn remove(&mut self, index: ManagedVecU8) -> Option<T> {
        let usize_index = usize::from(index);
        self.data.get(usize_index)?;
        let mut ret_value = self.data.get_mut(usize_index).and_then(|v| v.take());
        // Shrink the vector by removing all empty last cells.
        while let Some(None) = self.data.last() {
            self.data.pop();
        }
        if let Some(value) = &mut ret_value {
            self.some_len -= 1;
            value.set_idx(None);
        }
        ret_value
    }

    pub fn len(&self) -> ManagedVecU8 {
        self.some_len
    }

    pub fn is_empty(&self) -> bool {
        self.some_len == 0
    }

    // Implement Iter and IterMut to iterate over the used cells.
    pub fn into_iter(self) -> impl Iterator<Item = (ManagedVecU8, T)> {
        self.data
            .into_iter()
            .enumerate()
            .filter_map(|(index, cell)| cell.map(|value| (index.try_into().unwrap(), value)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ManagedVecU8, &T)> {
        self.data.iter().enumerate().filter_map(|(index, cell)| {
            cell.as_ref()
                .map(|value| (index.try_into().unwrap(), value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ManagedVecU8, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .filter_map(|(index, cell)| {
                cell.as_mut()
                    .map(|value| (index.try_into().unwrap(), value))
            })
    }
}

impl<T: ManagedElement> Default for ManagedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[test]

fn len() {
    struct TS {
        idx: Option<ManagedVecU8>,
        _value: u8,
    }

    impl TS {
        pub fn new(value: u8) -> Self {
            Self {
                idx: None,
                _value: value,
            }
        }
    }

    impl ManagedElement for TS {
        fn idx(&self) -> Option<ManagedVecU8> {
            self.idx
        }
        fn set_idx(&mut self, index: Option<ManagedVecU8>) {
            self.idx = index;
        }
    }

    // Initial simple check.
    let mut v1 = ManagedVec::<TS>::new();
    assert_eq!(v1.len(), 0);
    v1.push(TS::new(1));
    assert_eq!(v1.len(), 1);
    v1.push(TS::new(2));
    assert_eq!(v1.len(), 2);
    v1.remove(0);
    assert_eq!(v1.len(), 1);
    v1.remove(0);
    assert_eq!(v1.len(), 1);
    v1.remove(1);
    assert_eq!(v1.len(), 0);

    // Test removal of one element (test first, second, middle, before last and last case)
    for i in 0..=4 {
        let mut v1 = ManagedVec::<TS>::new();
        for j in 0..=4 {
            v1.push(TS::new(j));
        }
        assert_eq!(v1.len(), 5);
        let elem_removed = v1.remove(i);
        // Verify really removed (index in object should be None).
        assert_eq!(v1.len(), 4);
        assert!(elem_removed.is_some());
        let elem_removed = elem_removed.unwrap();
        assert!(elem_removed.idx().is_none());

        // Removing again should have no effect.
        let elem_removed2 = v1.remove(i);
        assert_eq!(v1.len(), 4);
        assert!(elem_removed2.is_none());
        assert!(elem_removed.idx().is_none());

        // Verify re-cycling of the index works.
        let elem_recycling = TS::new(5);
        let elem_recycling_idx = v1.push(elem_recycling);
        assert_eq!(v1.len(), 5);
        assert!(elem_recycling_idx.is_some());
        let elem_recycling_idx = elem_recycling_idx.unwrap();
        assert_eq!(elem_recycling_idx, i);
    }
}
