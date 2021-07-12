use crate::comp_structs::{dict_elem::DictElem, index_value_pair::IndexValuePair};
use crate::utility::{CHUNK_MAX_SIZE, ELEM_BYTES, MIN_OCCATIONS, VALUES};
use std::collections::HashMap;

pub struct Dictionary {
    pub elems: HashMap<usize, DictElem>,
    pub reverse_elems: HashMap<DictElem, usize>,
    pub least: IndexValuePair,
    pub coverage: u64,
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary {
            elems: HashMap::with_capacity(VALUES),
            reverse_elems: HashMap::with_capacity(VALUES),
            least: IndexValuePair::default(),
            coverage: CHUNK_MAX_SIZE,
        }
    }

    fn insert(&mut self, elem: &DictElem) {
        let index = self.elems.len();
        self.elems.insert(index, *elem);
        self.reverse_elems.insert(*elem, index);
    }

    fn replace_least(&mut self, elem: &DictElem) {
        self.reverse_elems.remove(self.elems.get(&self.least.index).unwrap());
        self.elems.insert(self.least.index, *elem);
        self.reverse_elems.insert(*elem, self.least.index);
    }

    fn redefine_least(&mut self) {
        let mut least = IndexValuePair {
            index: 0usize,
            value: u64::MAX,
        };

        for (index, elem) in self.elems.iter() {
            if least.value > elem.occurance {
                least.index = *index;
                least.value = elem.occurance;
            }
        }

        self.least = least;
    }

    fn full(&self) -> bool {
        self.elems.len() >= VALUES
    }

    pub fn consider(&mut self, elem: &DictElem) {
        match self.reverse_elems.get(elem) {
            Some(index) => {
                self.elems.insert(*index, *elem);
                if self.least.index == *index {
                    self.redefine_least();
                }
            }

            None => {
                if elem.occurance < MIN_OCCATIONS {
                    return;
                } else if !self.full() {
                    self.insert(elem);
                } else if elem.occurance > self.least.value {
                    self.replace_least(elem);
                    self.redefine_least();
                }
            }
        }
    }

    pub fn get(&self, index: u8) -> [u8; ELEM_BYTES] {
        self.elems.get(&(index as usize)).unwrap().data
    }

    pub fn get_index(&self, input: &[u8; ELEM_BYTES]) -> Option<u8> {
        if let Some(v) = self.reverse_elems.get(&DictElem::new(*input, 0)) {
            Some(*v as u8)
        } else {
            None
        }
    }

    pub fn purge_unused(&mut self) {
        let mut indexes_to_remove: Vec<usize> = vec![];

        for (index, elem) in self.elems.iter() {
            if elem.useage == 0 {
                self.reverse_elems.remove(elem);
                indexes_to_remove.push(*index);
            }
        }

        for index in indexes_to_remove.iter() {
            self.elems.remove(index);
        }

        let elems: Vec<DictElem> = self.elems.values().map(|v| *v).collect();
        self.elems.clear();
        self.reverse_elems.clear();

        for (index, elem) in elems.iter().enumerate() {
            self.elems.insert(index, *elem);
            self.reverse_elems.insert(*elem, index);
        }

        self.elems.shrink_to_fit();
        self.reverse_elems.shrink_to_fit();
    }

    pub fn len(&self) -> u8 {
        self.elems.len() as u8
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.elems.len() * ELEM_BYTES);

        for index in 0usize..self.elems.len() {
            out.extend(self.elems.get(&index).unwrap().data);
        }

        out
    }

    pub fn increment_usage(&mut self, index: u8) {
        if let Some(elem) = self.elems.get_mut(&(index as usize)) {
            elem.increment_usage();
        }
    }

    pub fn to_string(&self) -> String {
        let mut out_str = String::from(format!(
            "coverage: {} bytes. Elements: {}",
            self.coverage,
            self.elems.len()
        ));

        for (index, element) in self.elems.iter() {
            out_str.push_str(format!("\nElem {}: {}", index, element.to_string()).as_str());
        }

        out_str
    }
}
