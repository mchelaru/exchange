use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::{genericinstrumentlist::GenericInstrumentList, instrument::Instrument};

#[derive(Clone)]
pub struct MockInstrumentList {
    instrument_list: HashMap<u64, Rc<RefCell<Instrument>>>,
    iter_count: usize,
}

impl MockInstrumentList {}

impl Iterator for MockInstrumentList {
    type Item = Rc<RefCell<Instrument>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter_count < self.instrument_list.len() {
            let mut i = self.instrument_list.iter().skip(self.iter_count);
            self.iter_count += 1;
            match i.next() {
                Some((_, instrument)) => Some(instrument.clone()),
                None => None,
            }
        } else {
            None
        }
    }
}

// GenericInstrumentList implementation
impl GenericInstrumentList for MockInstrumentList {
    fn new() -> Self {
        Self {
            instrument_list: HashMap::new(),
            iter_count: 0,
        }
    }

    fn update_instrument(&mut self, i: &Instrument) {
        self.instrument_list
            .insert(i.get_id(), Rc::new(RefCell::new(i.clone())));
    }

    fn len(&self) -> usize {
        self.instrument_list.len()
    }

    fn contains(&self, id: u64) -> bool {
        self.instrument_list.contains_key(&id)
    }

    fn add(&mut self, id: u64, i_type: super::instrument::InstrumentType) {
        self.add_instrument(Instrument::new_fast(id, i_type));
    }

    fn add_instrument(&mut self, i: Instrument) -> Rc<RefCell<Instrument>> {
        let id = i.get_id();
        self.instrument_list.insert(id, Rc::new(RefCell::new(i)));
        self.instrument_list
            .get(&id)
            .expect("Can't insert instrument into market")
            .clone()
    }

    fn get(&self, id: u64) -> Option<Rc<RefCell<Instrument>>> {
        match self.instrument_list.get(&id) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }
}
