use std::cmp::Ordering;

#[derive(Eq, Default, Clone, Copy)]
pub struct IndexValuePair {
    pub index: usize,
    pub value: u64,
}

impl PartialEq for IndexValuePair {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for IndexValuePair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for IndexValuePair {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
