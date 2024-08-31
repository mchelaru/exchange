use std::error::Error;

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutionReport {
    pub participant: u64,
    pub order_id: u64,
    pub submitted_order_id: u64, // client_order_id for new order or exchange_order_id for modifies, cancels
    pub book: u64,
    pub quantity: u64,
    pub price: u64,
    pub flags: u16,
    pub side: u8,
    pub state: u8, // see OrderState
    pub session_id: u32,
    pub gateway_id: u8,
}

impl ExecutionReport {
    pub fn get_book(&self) -> u64 {
        self.book
    }

    pub fn get_price(&self) -> u64 {
        self.price
    }

    pub fn get_submitted_order_id(&self) -> u64 {
        self.submitted_order_id
    }

    pub fn get_order_id(&self) -> u64 {
        self.order_id
    }

    pub fn get_quantity(&self) -> u64 {
        self.quantity
    }
}

pub const EXECUTIONREPORT_SIZE: usize = std::mem::size_of::<ExecutionReport>();

impl Decoder<EXECUTIONREPORT_SIZE> for ExecutionReport {
    fn encode(self) -> [u8; EXECUTIONREPORT_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; EXECUTIONREPORT_SIZE]>(self) }
    }

    fn decode(buffer: [u8; EXECUTIONREPORT_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; EXECUTIONREPORT_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for ExecutionReport {
    fn message_type(&self) -> MsgType {
        MsgType::ExecutionReport
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_gateway_id(&self) -> u8 {
        self.gateway_id
    }

    fn get_session_id(&self) -> u32 {
        self.session_id
    }

    fn get_participant(&self) -> u64 {
        self.participant
    }
}

// FIXME: tests
