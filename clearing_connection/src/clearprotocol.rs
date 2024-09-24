// The implementation of the "Clear" Protocol

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::genericclearingprotocol::{GenericClearingProtocol, ProtocolSide};
use disseminator::disseminator::Disseminator;
use instruments::genericinstrumentlist::GenericInstrumentList;
use instruments::instrument::{Instrument, InstrumentState, InstrumentType};
use market::Market;

use super::genericclearingprotocol::ProcessError;

// the clear clearing protocol
const CLEAR_PROTOCOL_VERSION: u8 = 1;

const CLEAR_TYPE_HEARTBEAT: u16 = 0;
const CLEAR_TYPE_INSTRUMENT_UPDATE: u16 = 1;
const CLEAR_TYPE_INSTRUMENT_REQUEST: u16 = 2;
const CLEAR_TYPE_ALL_INSTRUMENTS_REQUEST: u16 = 3;

// for now this is hashmap. I want to change that
// key is instrument ID
type MarketCollection = Rc<RefCell<HashMap<u64, Market>>>;

pub struct ClearProtocol<T: GenericInstrumentList<Item = Rc<RefCell<Instrument>>>> {
    instrument_list: T,
    protocol_side: ProtocolSide,
    markets: MarketCollection,
    disseminator: Rc<RefCell<dyn Disseminator>>,
}

impl<T: GenericInstrumentList<Item = Rc<RefCell<Instrument>>>> ClearProtocol<T> {
    /// Initializes a struct with a `None` value for the
    /// `instrument_update_upcall` field.
    ///
    /// Returns:
    ///
    /// The `new` function is returning an instance of the struct that it is defined
    /// in.
    pub fn new(
        instrument_list: T,
        markets: MarketCollection,
        disseminator: Rc<RefCell<dyn Disseminator>>,
    ) -> Self {
        Self {
            instrument_list: instrument_list,
            protocol_side: ProtocolSide::Client,
            markets: markets,
            disseminator: disseminator,
        }
    }

    /// The function `process_one_data_entry` processes a data entry in a buffer and
    /// returns the number of bytes processed or an error.
    ///
    /// Arguments:
    ///
    /// * `buffer`: A slice of bytes that contains the data entry to be processed.
    ///
    /// Returns:
    ///
    /// The function `process_one_data_entry` returns a `Result<usize, ProcessError>`.
    fn process_one_data_entry(&mut self, buffer: &[u8]) -> Result<(Vec<u8>, usize), ProcessError> {
        let data_type =
            u16::from_le_bytes(buffer[0..2].try_into().expect("Invalid data type slice"));
        let data_len = u16::from_le_bytes(buffer[2..4].try_into().expect("Invalid data len slice"));
        let processed: usize = 4;
        if usize::from(data_len) + processed > buffer.len() {
            return Ok((vec![], 0));
        }
        match data_type {
            CLEAR_TYPE_HEARTBEAT => Ok((vec![], processed)),
            CLEAR_TYPE_INSTRUMENT_UPDATE => {
                if processed + data_len as usize > buffer.len() {
                    Ok((vec![], 0)) // too short, will process later
                } else {
                    let instrument_id = u64::from_le_bytes(
                        buffer[4..12].try_into().expect("Invalid instrument ID"),
                    );
                    let instrument_type = match buffer[12].to_le() {
                        0 => InstrumentType::Share,
                        1 => InstrumentType::OptionCall,
                        2 => InstrumentType::OptionPut,
                        3 => InstrumentType::Future,
                        4 => InstrumentType::Warrant,
                        _ => InstrumentType::Share,
                    };
                    let instrument_state = match buffer[13].to_le() {
                        0 => InstrumentState::Trading,
                        1 => InstrumentState::Closed,
                        2 => InstrumentState::Auction,
                        _ => InstrumentState::Closed,
                    };

                    if self.protocol_side == ProtocolSide::Client {
                        // update the specific instrument
                        let percentage_bands = buffer[14].to_le();
                        let percentage_variation_allowed = buffer[15].to_le();
                        //extract the name
                        let mut v = buffer[16..].to_vec();
                        v.truncate(data_len as usize - 12);
                        let name = String::from_utf8(v).unwrap();
                        let instrument = Instrument::new(
                            instrument_id,
                            &name,
                            instrument_type,
                            instrument_state,
                            percentage_bands,
                            percentage_variation_allowed,
                        );
                        let inserted_instrument = self.instrument_list.add_instrument(instrument);

                        if let Some(m) = self.markets.borrow().get(&instrument_id) {
                            m.instrument_updated();
                            return Ok((vec![], processed + data_len as usize)); // we do this just to drop the borrow
                        }
                        self.markets.borrow_mut().insert(
                            instrument_id,
                            Market::new(inserted_instrument, self.disseminator.clone()),
                        );
                    }
                    Ok((vec![], processed + data_len as usize))
                }
            }
            CLEAR_TYPE_INSTRUMENT_REQUEST => {
                if processed + 8 > buffer.len() {
                    Ok((vec![], 0))
                } else {
                    let instrument_id = u64::from_le_bytes(
                        buffer[4..12].try_into().expect("Invalid instrument ID"),
                    );
                    match self.instrument_list.get(instrument_id) {
                        Some(instrument) => {
                            let response =
                                self.prepare_instrument_update_response(&instrument.borrow());
                            Ok((response, processed + 8))
                        }
                        None => Ok((vec![], processed + 8)),
                    }
                }
            }
            CLEAR_TYPE_ALL_INSTRUMENTS_REQUEST => {
                let response = self
                    .instrument_list
                    .clone()
                    .map(|i| self.prepare_instrument_update_response(&i.borrow()))
                    .reduce(|mut acc, mut i| {
                        (&mut acc).append(&mut i);
                        acc
                    })
                    .unwrap_or_default(); // default in case there is no instrument
                Ok((response, processed))
            }
            _ => Err(ProcessError::new("Invalid type")),
        }
    }
}

impl<T: GenericInstrumentList<Item = Rc<RefCell<Instrument>>>> GenericClearingProtocol
    for ClearProtocol<T>
{
    /// Takes a buffer of bytes, checks for a valid header,
    /// processes data entries in the buffer, and returns the number of bytes
    /// processed.
    ///
    /// Arguments:
    ///
    /// * `buffer`: A slice of bytes that represents the data to be processed.
    ///
    /// Returns:
    ///
    /// The function `process` returns a `Result<usize, ProcessError>`
    /// representing the number of bytes that have been processed.
    fn process(&mut self, buffer: &[u8]) -> Result<(Vec<u8>, usize), ProcessError> {
        if buffer.len() < 8 {
            return Ok((vec![], 0));
        }

        if buffer[0] != b'C' || buffer[1] != b'P' {
            return Err(ProcessError::new("Invalid header"));
        }
        if buffer[2].to_le() != CLEAR_PROTOCOL_VERSION {
            return Err(ProcessError::new("Invalid protocol version"));
        }

        let mut entries = buffer[3].to_le();
        let mut processed_bytes = 4; // technical header len
        let mut response = vec![];
        while buffer.len() - processed_bytes >= 4 && entries > 0 {
            let (mut one_response, pbytes) =
                self.process_one_data_entry(&buffer[processed_bytes..])?;
            entries -= 1;
            if pbytes == 0 {
                break;
            }
            processed_bytes += pbytes;
            response.append(&mut one_response);
        }
        Ok((response, processed_bytes))
    }

    // Generic messages ready to send out as replies
    fn prepare_heartbeat(&self) -> Vec<u8> {
        vec![b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, 0, 0, 0, 0]
    }

    fn prepare_all_instrument_request(&self) -> Vec<u8> {
        vec![b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, 3, 0, 0, 0]
    }

    fn prepare_instrument_update_response(&self, instrument: &Instrument) -> Vec<u8> {
        let length: u16 = (12 + instrument.get_name().len())
            .try_into()
            .unwrap_or_default();
        if length == 0 {
            eprintln!(
                "Error preparing update for instrument {}",
                instrument.get_id()
            );
            return vec![];
        }

        let mut r = vec![
            b'C',
            b'P',
            CLEAR_PROTOCOL_VERSION,
            1,
            1,
            0,
            length.to_le_bytes()[0],
            length.to_le_bytes()[1],
        ];
        // Attention, this fixes the clearing encoding to the feed encoding
        r.append(&mut instrument.encode());
        r
    }

    fn clone_instrument_list(&self) -> Vec<Instrument> {
        self.instrument_list
            .clone()
            .map(|x| x.borrow().clone())
            .collect()
    }

    fn add_instrument(&mut self, i: Instrument) {
        self.instrument_list.add_instrument(i);
    }

    fn set_protocol_side(&mut self, side: ProtocolSide) {
        self.protocol_side = side;
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use disseminator::mockdisseminator::MockDisseminator;
    use instruments::genericinstrumentlist::GenericInstrumentList;
    use instruments::instrument::{Instrument, InstrumentState, InstrumentType};
    use instruments::instrumentlist::InstrumentList;
    use instruments::mockinstrumentlist::MockInstrumentList;
    use market::Market;

    use super::ClearProtocol;
    use super::CLEAR_PROTOCOL_VERSION;
    use crate::clearprotocol::{CLEAR_TYPE_ALL_INSTRUMENTS_REQUEST, CLEAR_TYPE_INSTRUMENT_UPDATE};
    use crate::genericclearingprotocol::GenericClearingProtocol;

    #[test]
    fn instrument_update_no_upcall() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            MockInstrumentList::new(),
            markets,
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            8, 7, 6, 5, 4, 3, 2, 1, 0, 2, 20, 25,
        ];

        let v = target.process(&packet);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, packet.len());
    }

    #[test]
    fn one_instrument_update() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            MockInstrumentList::new(),
            markets,
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12 + 3, 0, // Instrument update
            8, 7, 6, 5, 4, 3, 2, 1, 0, 2, 20, 25,
            b'A', b'B', b'C'
        ];

        let v = target.process(&packet);

        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, packet.len());

        let i_list = target.clone_instrument_list();
        assert_eq!(1, i_list.len());
        let ins = &i_list[0];
        assert_eq!(ins.get_id(), 0x0102030405060708);
        assert_eq!(ins.get_type(), InstrumentType::Share);
        assert_eq!(ins.get_state(), InstrumentState::Auction);
        assert_eq!(ins.get_percentage_bands(), 20);
        assert_eq!(ins.get_percentage_variation_allowed(), 25);
        assert_eq!("ABC", ins.get_name());
    }

    #[test]
    fn two_instrument_updates() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            MockInstrumentList::new(),
            markets,
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 2, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            8, 7, 6, 5, 4, 3, 2, 1, 0, 2, 20, 25, // first instrument
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            9, 8, 6, 5, 4, 3, 2, 1, 0, 2, 20, 25, // second instrument
        ];

        let v = target.process(&packet);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, packet.len());

        let mut i_list = target.clone_instrument_list();
        i_list.sort_by_cached_key(|i| i.get_id());
        assert_eq!(2, i_list.len());

        let ins = &i_list[0];
        assert_eq!(ins.get_id(), 0x0102030405060708);
        assert_eq!(ins.get_type(), InstrumentType::Share);
        assert_eq!(ins.get_state(), InstrumentState::Auction);
        assert_eq!(ins.get_percentage_bands(), 20);
        assert_eq!(ins.get_percentage_variation_allowed(), 25);

        let ins = &i_list[1];
        assert_eq!(ins.get_id(), 0x0102030405060809);
        assert_eq!(ins.get_type(), InstrumentType::Share);
        assert_eq!(ins.get_state(), InstrumentState::Auction);
        assert_eq!(ins.get_percentage_bands(), 20);
        assert_eq!(ins.get_percentage_variation_allowed(), 25);
    }

    #[test]
    fn one_incomplete_instrument_update() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            MockInstrumentList::new(),
            markets,
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            8, 7, 6, 5, 4, 3, 2, 1, 0, 2,
        ];

        let v = target.process(&packet);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, 4);
        assert_eq!(0, target.clone_instrument_list().len());
    }

    #[test]
    fn one_complete_one_incomplete_instrument_update() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            MockInstrumentList::new(),
            markets,
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            8, 7, 6, 5, 4, 3, 2, 1, 0, 2, 20, 25, // first instrument
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            9, 8, 6, 5, 4, 3, 2, 1, 0, 2, // second incomplete instrument
        ];

        let v = target.process(&packet);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, 4 + 4 + 12);

        let i_list = target.clone_instrument_list();
        assert_eq!(1, i_list.len());

        let ins = &i_list[0];
        assert_eq!(ins.get_id(), 0x0102030405060708);
        assert_eq!(ins.get_type(), InstrumentType::Share);
        assert_eq!(ins.get_state(), InstrumentState::Auction);
        assert_eq!(ins.get_percentage_bands(), 20);
        assert_eq!(ins.get_percentage_variation_allowed(), 25);
    }

    /// tests if an update received on the wire that changes the state of the instrument
    /// also changes the field into the original instrument that is shared between
    /// components (market, instrument_lists, clearing etc.)
    #[test]
    fn instrument_update_propagates() {
        let instrument = Instrument::new_fast(0x0102030405060708, InstrumentType::OptionPut);
        assert_eq!(InstrumentState::Closed, instrument.get_state());

        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            InstrumentList::new(),
            markets.clone(),
            Rc::new(RefCell::new(MockDisseminator::new())),
        );
        let instrument_ref = target.instrument_list.add_instrument(instrument);
        markets.borrow_mut().insert(
            instrument_ref.borrow().get_id(),
            Market::new(
                instrument_ref.clone(),
                Rc::new(RefCell::new(MockDisseminator::new())),
            ),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_INSTRUMENT_UPDATE as u8, 0, 12, 0, // Instrument update, Len: 12
            8, 7, 6, 5, 4, 3, 2, 1, 2, 0, 20, 25, // update the instrument to trading
        ];

        let v = target.process(&packet);

        assert!(v.is_ok());
        assert_eq!(v.unwrap().1, packet.len());

        assert_eq!(
            InstrumentState::Trading,
            instrument_ref.borrow().get_state()
        );
        assert_eq!(
            InstrumentState::Trading,
            markets.borrow()[&instrument_ref.borrow().get_id()].get_state()
        );
    }

    #[test]
    fn request_all_instruments() {
        let instrument1 = Instrument::new_fast(0x0102030405060708, InstrumentType::OptionPut);
        let instrument2 = Instrument::new_fast(0x0502010405060708, InstrumentType::OptionCall);
        assert_eq!(InstrumentState::Closed, instrument1.get_state());
        assert_eq!(InstrumentState::Closed, instrument2.get_state());

        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            InstrumentList::new(),
            markets.clone(),
            Rc::new(RefCell::new(MockDisseminator::new())),
        );
        let _ = target.instrument_list.add_instrument(instrument1);
        let instrument_ref = target.instrument_list.add_instrument(instrument2);
        markets.borrow_mut().insert(
            instrument_ref.borrow().get_id(),
            Market::new(
                instrument_ref.clone(),
                Rc::new(RefCell::new(MockDisseminator::new())),
            ),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_ALL_INSTRUMENTS_REQUEST as u8, 0, 0, 0, // All instruments request, Len: 0
        ];

        let v = target.process(&packet);

        assert!(v.is_ok());
        assert_eq!(v.as_ref().unwrap().1, packet.len());

        assert_eq!(
            (8 + 12) * target.instrument_list.len(), // 8 header + 12 data
            v.as_ref().unwrap().0.len()
        );
    }

    #[test]
    fn request_all_instruments_but_no_instruments_resuts_in_empty_response() {
        let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
        let mut target = ClearProtocol::new(
            InstrumentList::new(),
            markets.clone(),
            Rc::new(RefCell::new(MockDisseminator::new())),
        );

        #[rustfmt::skip]
        let packet = [
            b'C', b'P', CLEAR_PROTOCOL_VERSION, 1, // technical header
            CLEAR_TYPE_ALL_INSTRUMENTS_REQUEST as u8, 0, 0, 0, // All instruments request, Len: 0
        ];

        let v = target.process(&packet);

        assert!(v.is_ok());
        assert_eq!(v.as_ref().unwrap().1, packet.len());

        assert_eq!(0, v.as_ref().unwrap().0.len());
    }
}
