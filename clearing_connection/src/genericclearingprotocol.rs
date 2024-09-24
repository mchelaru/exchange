use core::fmt;
use std::{error::Error, str};

use instruments::instrument::Instrument;

#[derive(Debug)]
pub struct ProcessError {
    error_text: String,
}

impl ProcessError {
    pub fn new(v: &str) -> Self {
        Self {
            error_text: v.to_string(),
        }
    }
}

impl Error for ProcessError {}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error_text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolSide {
    Client,
    Server,
}

pub trait GenericClearingProtocol {
    fn process(&mut self, buffer: &[u8]) -> Result<(Vec<u8>, usize), ProcessError>;
    fn clone_instrument_list(&self) -> Vec<Instrument>;
    fn add_instrument(&mut self, i: Instrument);

    // Generic messages
    fn prepare_heartbeat(&self) -> Vec<u8>;
    fn prepare_all_instrument_request(&self) -> Vec<u8>;
    fn prepare_instrument_update_response(&self, instrument: &Instrument) -> Vec<u8>;
    fn set_protocol_side(&mut self, side: ProtocolSide);
}
