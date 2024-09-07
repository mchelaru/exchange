use std::{cell::RefCell, rc::Rc};

use super::instrument::{Instrument, InstrumentType};

pub trait GenericInstrumentList: Iterator + Clone + Sized {
    fn new() -> Self
    where
        Self: Sized;
    fn add_instrument(&mut self, i: Instrument) -> Rc<RefCell<Instrument>>;
    fn update_instrument(&mut self, i: &Instrument);
    fn get(&self, id: u64) -> Option<Rc<RefCell<Instrument>>>;
    fn len(&self) -> usize;
    fn contains(&self, id: u64) -> bool;
    fn add(&mut self, id: u64, itype: InstrumentType);
}
