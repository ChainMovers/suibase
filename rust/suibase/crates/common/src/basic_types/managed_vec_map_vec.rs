use std::collections::HashMap;

// Managed Vector-Map-Vector
//
// Similar to ManagedVec, this container is intended for high number of fast lookups on
// a limited number of elements that are rarely added/removed.
//
// ManagedVevMapVect<T> provides a 3 level indexing to access 'T' elements:
//  - Index 1 (u8 key): A vector of index2_elements.
//  - Index 2 (string key): Each index2_elements is a map of <string,index3_elements>
//  - Index 3 (u8 key): Each index3_elements is a vector of 'T' elements.
//
// Each 'T' element also get assigned a ManagedVecU16 index [0..65535]
//
// This design makes possible:
//   - O(1) lookup for 'T' elements with the ManagedVec16 index.
//
//   - Slower (but still fast) lookup of a 'T' element with a key tuple (index1,index2,index3)
//
//   - Iterators with key tuples (index1), (index1,index2) and (index1,index2,index3).
//
// Some limitation:
//    - first level key must be [0..255]
//    - second level key must be a string
//    - third level key must be [0..255]
//    - Stores max 65536 'T' elements.
//
// It is crucial for performance to keep first and third level index range as low
// as possible (ideally < 30).
//
// Example of use:
//   - Map a key tuple (service_idx, host_addr, sla_idx) into a 'T' element.
//     Typically, the service_idx will be <= 30, and sla_idx <= 1
//     (SLA is for "Service Level Agreement").
//
//     Each 'T' element will get assigned a unique value [0..65535] that can
//     be used for fast array direct access.
//
use super::AutoSizeVec;
use super::{ManagedElement16, ManagedVec16, ManagedVecU16};

pub struct ManagedVecMapVec<T> {
    lookup: AutoSizeVec<Level1Element>,
    managed_vec: ManagedVec16<T>,
}

// Internal structure levels.
struct Level1Element {
    data: HashMap<String, Level2Element>,
}
impl Default for Level1Element {
    fn default() -> Self {
        Level1Element {
            data: HashMap::new(),
        }
    }
}

struct Level2Element {
    data: AutoSizeVec<Level3Element>,
}
impl Default for Level2Element {
    fn default() -> Self {
        Level2Element {
            data: AutoSizeVec::new(),
        }
    }
}

struct Level3Element {
    // The index used for both the ManagedVecMapVec<T>::managed_vec
    // and the user defined loosely coupled AutosizeVecMapVec.
    idx: ManagedVecU16,
}

impl Default for Level3Element {
    fn default() -> Self {
        Level3Element { idx: 0 }
    }
}

impl<T: ManagedElement16> ManagedVecMapVec<T> {
    pub fn new() -> Self {
        Self {
            lookup: AutoSizeVec::new(),
            managed_vec: ManagedVec16::new(),
        }
    }

    // That is the only time the index is set and returned.
    // TODO Verify handling of out of range index.
    pub fn push(
        &mut self,
        value: T,
        index1: u8,
        index2: String,
        index3: u8,
    ) -> Option<ManagedVecU16> {
        // Push T into the managed_vec and get its index.
        let managed_idx = self.managed_vec.push(value);

        if managed_idx.is_none() {
            return None;
        }

        // Lookup to get a mut on the Level3Element. (it is created if does not exist).
        // We will store in it the managed_idx of the newly pushed value.

        let level1_element = self.lookup.get_mut(index1);
        let mut level2_element = level1_element.data.get_mut(&index2);
        if level2_element.is_none() {
            // Create the level2_element
            level1_element
                .data
                .insert(index2.clone(), Level2Element::default());
            level2_element = level1_element.data.get_mut(&index2);
        }
        let level2_element = level2_element.unwrap();
        let level3_element = level2_element.data.get_mut(index3);
        level3_element.idx = managed_idx.unwrap();

        managed_idx
    }

    // TODO Verify getting OOB have no effect.
    pub fn get(&self, index: ManagedVecU16) -> Option<&T> {
        self.managed_vec.get(index)
    }

    // TODO Verify getting OOB have no effect.
    pub fn get_mut(&mut self, index: ManagedVecU16) -> Option<&mut T> {
        self.managed_vec.get_mut(index)
    }

    // This free the cells for re-use. If a push is done, it
    // might re-use that cell (the same index).
    //
    // TODO Verify remove OOB have no effect.
    pub fn remove(&mut self, index: ManagedVecU16) -> Option<T> {
        self.managed_vec.remove(index)
    }

    pub fn len(&self) -> ManagedVecU16 {
        self.managed_vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.managed_vec.is_empty()
    }

    // Implement Iter and IterMut to iterate over the used cells.
    pub fn into_iter(self) -> impl Iterator<Item = (ManagedVecU16, T)> {
        self.managed_vec.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ManagedVecU16, &T)> {
        self.managed_vec.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ManagedVecU16, &mut T)> {
        self.managed_vec.iter_mut()
    }
}

impl<T: ManagedElement16> Default for ManagedVecMapVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[test]

fn len() {
    struct TS {
        idx: Option<ManagedVecU16>,
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

    impl ManagedElement16 for TS {
        fn idx(&self) -> Option<ManagedVecU16> {
            self.idx
        }
        fn set_idx(&mut self, index: Option<ManagedVecU16>) {
            self.idx = index;
        }
    }

    // Initial simple check.
    let mut v1 = ManagedVecMapVec::<TS>::new();
    assert_eq!(v1.len(), 0);
    v1.push(TS::new(1), 0, "a".to_string(), 0);
    assert_eq!(v1.len(), 1);
    v1.push(TS::new(2), 10, "a".to_string(), 1);
    assert_eq!(v1.len(), 2);
    v1.remove(0);
    assert_eq!(v1.len(), 1);
    v1.remove(0);
    assert_eq!(v1.len(), 1);
    v1.remove(1);
    assert_eq!(v1.len(), 0);

    // Test removal of one element (test first, second, middle, before last and last case)
    for i in 0..=4 {
        let mut v1 = ManagedVecMapVec::<TS>::new();
        for j in 0..=4 {
            v1.push(TS::new(j), i, "a".to_string(), j);
        }
        assert_eq!(v1.len(), 5);
        let elem_removed = v1.remove(i as u16);
        // Verify really removed (index in object should be None).
        assert_eq!(v1.len(), 4);
        assert!(elem_removed.is_some());
        let elem_removed = elem_removed.unwrap();
        assert!(elem_removed.idx().is_none());

        // Removing again should have no effect.
        let elem_removed2 = v1.remove(i as u16);
        assert_eq!(v1.len(), 4);
        assert!(elem_removed2.is_none());
        assert!(elem_removed.idx().is_none());

        // Verify re-cycling of the index works.
        let elem_recycling = TS::new(5);
        let elem_recycling_idx = v1.push(elem_recycling, i, "a".to_string(), 0);
        assert_eq!(v1.len(), 5);
        assert!(elem_recycling_idx.is_some());
        let elem_recycling_idx = elem_recycling_idx.unwrap();
        assert_eq!(elem_recycling_idx, i as u16);
    }
}
