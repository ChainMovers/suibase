use super::ManagedVecUSize;

pub struct AutoSizeVec<T> {
    data: Vec<Option<T>>,
}

impl<T: Default> AutoSizeVec<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    // get and get_mut that creates a default T object
    // if the index does not have an element at the index.
    //
    // Intended use is for when every element in a "follower"
    // AutoSizeVec has a 1:1 relationship with a "leader" ManagedVec.
    //
    // This allow multiple "loosely coupled" system to synchronized with
    // very fast lookup using the leader key/index.
    //
    pub fn get(&mut self, leader_index: ManagedVecUSize) -> &T {
        let usize_index = usize::from(leader_index);
        // Extend the size of self.data if leader_index is out-of-bounds of self.data
        if usize_index > self.data.len() {
            self.data.resize_with(usize_index, || None);
        }
        if usize_index >= self.data.len() {
            self.data.push(Some(T::default()));
        }
        if self.data[usize_index].is_none() {
            self.data[usize_index] = Some(T::default());
        }
        self.data.get(usize_index).and_then(|v| v.as_ref()).unwrap()
    }

    pub fn get_mut(&mut self, leader_index: ManagedVecUSize) -> &mut T {
        let usize_index = usize::from(leader_index);
        // Extend the size of self.data if leader_index is out-of-bounds of self.data
        if usize_index > self.data.len() {
            self.data.resize_with(usize_index, || None);
        }
        if usize_index >= self.data.len() {
            self.data.push(Some(T::default()));
        }
        if self.data[usize_index].is_none() {
            self.data[usize_index] = Some(T::default());
        }
        self.data
            .get_mut(usize_index)
            .and_then(|v| v.as_mut())
            .unwrap()
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

// Implement Debug for AutoSizeVec.
// Iterate only the used cell and call Display/Debug on the value.
impl<T: std::fmt::Debug + Default> std::fmt::Debug for AutoSizeVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, value) in self.iter() {
            writeln!(f, "{}: {:?}", index, value)?;
        }
        Ok(())
    }
}

#[test]

fn one_element() {
    struct TS {
        pub value: u32,
    }

    impl TS {
        pub fn new(value: u32) -> Self {
            Self { value }
        }
    }

    impl Default for TS {
        fn default() -> Self {
            Self { value: u32::MAX }
        }
    }

    // Initialize with default one element, at various index.
    let mut v1 = AutoSizeVec::<TS>::new();
    for idx in (0..=5).chain(u8::MAX - 2..=u8::MAX) {
        let el1 = v1.get(idx);
        assert_eq!(el1.value, u32::MAX);
    }

    // Modify these same elements with their own position.
    for idx in (0..=5).chain(u8::MAX - 2..=u8::MAX) {
        let el1 = v1.get_mut(idx);
        assert_eq!(el1.value, u32::MAX);
        el1.value = idx as u32;
    }

    // Read back to verify modifications preserve in the Vec.
    for idx in (0..=5).chain(u8::MAX - 2..=u8::MAX) {
        let el1 = v1.get(idx);
        assert_eq!(el1.value, idx as u32);
    }
}
