use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::genericinstrumentlist::GenericInstrumentList;
use crate::instrument::{Instrument, InstrumentType};

/// This is a simple Instrument List implementation based on a hash map
///
/// # Example:
///
/// ```
/// # use instruments::instrumentlist::InstrumentList;
/// # use instruments::instrument::InstrumentType;
/// # use crate::instruments::genericinstrumentlist::GenericInstrumentList;
/// let mut instrument_list = InstrumentList::new();
/// instrument_list.add(100, InstrumentType::Share);
/// assert_eq!(1, instrument_list.len());
/// ```
///
#[derive(Debug, Clone)]
pub struct InstrumentList {
    instrument_list: HashMap<u64, Rc<RefCell<Instrument>>>,
    iter_count: usize,
}

impl Iterator for InstrumentList {
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

impl InstrumentList {}

// GenericInstrumentList trait
impl GenericInstrumentList for InstrumentList {
    fn new() -> Self {
        Self {
            instrument_list: HashMap::new(),
            iter_count: 0,
        }
    }

    fn update_instrument(&mut self, i: &Instrument) {
        self.add(i.get_id(), i.get_type());
    }

    fn len(&self) -> usize {
        self.instrument_list.len()
    }

    fn contains(&self, id: u64) -> bool {
        self.instrument_list.contains_key(&id)
    }

    fn get(&self, id: u64) -> Option<Rc<RefCell<Instrument>>> {
        match self.instrument_list.get(&id) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }

    /// this is a quick hackish function used mostly in tests
    /// Use add_instrument instead
    fn add(&mut self, id: u64, itype: InstrumentType) {
        self.instrument_list
            .insert(id, Rc::new(RefCell::new(Instrument::new_fast(id, itype))));
    }

    /// adds an instrument to the list.
    /// If the ID is already present, then the previous entry is replaced
    fn add_instrument(&mut self, i: Instrument) -> Rc<RefCell<Instrument>> {
        let id = i.get_id();
        match self.contains(id) {
            false => {
                let r = Rc::new(RefCell::new(i));
                self.instrument_list.insert(id, r.clone());
                r
            }
            true => {
                let existing = self.get(i.get_id()).unwrap();
                existing.borrow_mut().clone_from(&i);
                return existing.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::instrument::InstrumentType;

    use super::{GenericInstrumentList, InstrumentList};

    #[test]
    fn dont_duplicate() {
        let mut target = InstrumentList::new();
        target.add(100, InstrumentType::Share);
        target.add(200, InstrumentType::Share);
        target.add(100, InstrumentType::Share);

        assert_eq!(2, target.len());
        assert!(target.contains(100));
        assert!(target.contains(200));
        assert_eq!(false, target.contains(300));
    }

    #[test]
    fn iterable() {
        let mut target = InstrumentList::new();
        let instruments = vec![
            (100, InstrumentType::Share),
            (200, InstrumentType::OptionCall),
            (300, InstrumentType::Future),
        ];
        instruments
            .iter()
            .map(|(a, b)| target.add(a.clone(), b.clone()))
            .for_each(drop);

        let mut response: Vec<_> = target.collect();
        response.sort_unstable_by_key(|i| i.borrow().get_id());

        let mut expected = instruments.iter();
        let mut actual = response.iter();

        while let Some(ex) = expected.next() {
            let ac = actual.next().unwrap();
            assert_eq!(ex.0, ac.borrow().get_id());
            assert_eq!(ex.1, ac.borrow().get_type());
        }
    }

    #[test]
    fn iterable_twice() {
        let mut target = InstrumentList::new();
        let instruments = vec![
            (100, InstrumentType::Share),
            (200, InstrumentType::OptionCall),
            (300, InstrumentType::Future),
        ];
        instruments
            .iter()
            .map(|(a, b)| target.add(a.clone(), b.clone()))
            .for_each(drop);

        let mut response: Vec<_> = target.clone().collect();
        response.sort_unstable_by_key(|i| i.borrow().get_id());
        assert_eq!(instruments.len(), response.len());

        let mut expected = instruments.iter();
        let mut actual = response.iter();

        while let Some(ex) = expected.next() {
            let ac = actual.next().unwrap();
            assert_eq!(ex.0, ac.borrow().get_id());
            assert_eq!(ex.1, ac.borrow().get_type());
        }

        // Second time
        let mut response: Vec<_> = target.collect();
        response.sort_unstable_by_key(|i| i.borrow().get_id());
        assert_eq!(instruments.len(), response.len());

        let mut expected = instruments.iter();
        let mut actual = response.iter();

        while let Some(ex) = expected.next() {
            let ac = actual.next().unwrap();
            assert_eq!(ex.0, ac.borrow().get_id());
            assert_eq!(ex.1, ac.borrow().get_type());
        }
    }
}
