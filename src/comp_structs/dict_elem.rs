use crate::utility;
use std::hash::{Hash, Hasher};

#[derive(Eq, Clone, Copy)]
pub struct DictElem {
    pub data: [u8; utility::ELEM_BYTES],
    pub occurance: u64,
    pub useage: u64,
}

impl Hash for DictElem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data[0].hash(state);
        self.data[1].hash(state);
    }
}

impl PartialEq for DictElem {
    fn eq(&self, other: &Self) -> bool {
        self.data[0] == other.data[0] && self.data[1] == other.data[1]
    }
}

impl DictElem {
    pub fn new(arr: [u8; utility::ELEM_BYTES], occ: u64) -> Self {
        DictElem {
            data: arr,
            occurance: occ,
            useage: 0,
        }
    }

    pub fn eq_array(&self, o: &[u8; utility::ELEM_BYTES]) -> bool {
        self.data[0] == o[0] && self.data[1] == o[1]
    }

    pub fn set_occurrence(&mut self, occ: u64) {
        self.occurance = occ;
    }

    pub fn increment_usage(&mut self) {
        self.useage += 1;
    }

    pub fn to_string(&self) -> String {
        let t0 = utility::u8_to_string(self.data[0]);
        let t1 = utility::u8_to_string(self.data[1]);

        format!(" [{}, {}]: {} occasions", t0, t1, self.occurance)
    }
}
