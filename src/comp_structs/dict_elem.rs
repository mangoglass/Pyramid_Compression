use crate::utility;

#[derive(Eq, Clone, Copy)]
pub struct DictElem {
    pub data: [u8; utility::ELEM_BYTES],
    pub occurance: u64,
    pub useage: u64,
}

impl PartialEq for DictElem {
    fn eq(&self, other: &Self) -> bool {
        for (self_elem, other_elem) in self.data.iter().zip(other.data.iter()) {
            if self_elem != other_elem {
                return false;
            }
        }
        true
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

    pub fn set_occurance(&mut self, occ: u64) {
        self.occurance = occ;
    }

    pub fn increment_useage(&mut self) {
        self.useage += 1;
    }

    pub fn to_string(&self) -> String {
        let t0 = utility::u8_to_string(self.data[0]);
        let t1 = utility::u8_to_string(self.data[1]);

        format!(" [{}, {}]: {} occasions", t0, t1, self.occurance)
    }
}
