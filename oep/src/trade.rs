use std::error::Error;

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};

/// This message is here only for the feed disseminator
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Trade {
    pub bid_order_id: u64,
    pub ask_order_id: u64,
    pub price: u64,
    pub quantity: u64,
}

pub const TRADE_SIZE: usize = std::mem::size_of::<Trade>();

impl Decoder<TRADE_SIZE> for Trade {
    fn encode(self) -> [u8; TRADE_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; TRADE_SIZE]>(self) }
    }

    fn decode(buffer: [u8; TRADE_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; TRADE_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for Trade {
    fn message_type(&self) -> MsgType {
        MsgType::Trade
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_gateway_id(&self) -> u8 {
        panic!("No gateway in a trade message")
    }

    fn get_session_id(&self) -> u32 {
        panic!("No session in a trade message")
    }

    fn get_participant(&self) -> u64 {
        panic!("No participant in a trade message")
    }
}
