use crate::comp_structs::{dict_elem::DictElem, index_value_pair::IndexValuePair};
use crate::utility::{CHUNK_MAX_SIZE, ELEM_BYTES, MIN_OCCATIONS, VALUES};

pub struct Dictionary {
    pub elems: Vec<DictElem>,
    pub least: IndexValuePair,
    pub coverage: u64,
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary {
            elems: vec![],
            least: IndexValuePair::default(),
            coverage: CHUNK_MAX_SIZE,
        }
    }

    fn push(&mut self, elem: &DictElem) {
        self.elems.push(*elem);
    }

    fn replace(&mut self, elem: &DictElem, index: usize) {
        self.elems[index] = *elem;
    }

    fn redefine_least(&mut self) {
        let mut least = IndexValuePair {
            index: 0usize,
            value: std::u64::MAX,
        };

        for (index, elem) in self.elems.iter().enumerate() {
            if least.value > elem.occurance {
                least.index = index;
                least.value = elem.occurance;
            }
        }

        self.least = least;
    }

    fn full(&self) -> bool {
        self.elems.len() >= VALUES
    }

    pub fn consider(&mut self, other_elem: &DictElem) {
        for (index, elem) in self.elems.iter_mut().enumerate() {
            if elem == other_elem {
                elem.set_occurance(other_elem.occurance);
                if index == self.least.index {
                    self.redefine_least();
                }

                return;
            }
        }

        if other_elem.occurance < MIN_OCCATIONS {
            return;
        } else if !self.full() {
            self.push(other_elem);
        } else if other_elem.occurance > self.least.value {
            self.replace(other_elem, self.least.index);
            self.redefine_least();
        }
    }

    pub fn get(&self, index: u8) -> [u8; ELEM_BYTES] {
        self.elems[index as usize].data
    }

    pub fn get_index(&self, input: &[u8; ELEM_BYTES]) -> Option<u8> {
        for (index, elem) in self.elems.iter().enumerate() {
            if elem.eq_array(input) {
                return Some(index as u8);
            }
        }

        None
    }

    pub fn purge_unused(&mut self) {
        let mut indexes_to_remove: Vec<usize> = vec![];

        for (index, elem) in self.elems.iter().enumerate().rev() {
            if elem.useage == 0 {
                indexes_to_remove.push(index);
            }
        }

        for index in indexes_to_remove.iter() {
            self.elems.remove(*index);
        }
    }

    pub fn len(&self) -> u8 {
        self.elems.len() as u8
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.elems.len());

        for element in self.elems.iter() {
            out.extend(&element.data);
        }

        out
    }

    pub fn increment_useage(&mut self, index: u8) {
        self.elems[index as usize].increment_useage();
    }

    pub fn to_string(&self) -> String {
        let mut out_str = String::from(format!(
            "coverage: {} bytes. Elements: {}",
            self.coverage,
            self.elems.len()
        ));

        for (index, element) in self.elems.iter().enumerate() {
            out_str.push_str(format!("\nElem {}: {}", index, element.to_string()).as_str());
        }

        out_str
    }
}
