use std::error::Error;

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Modify {
    pub participant: u64,
    pub order_id: u64,
    pub book_id: u64,
    pub quantity: u64,
    pub price: u64,
    pub side: u8,
    pub gateway_id: u8,
    pub session_id: u32,
}

pub const MODIFY_SIZE: usize = std::mem::size_of::<Modify>();

impl Modify {
    pub fn get_side(&self) -> u8 {
        self.side
    }
}

impl Decoder<MODIFY_SIZE> for Modify {
    fn encode(self) -> [u8; MODIFY_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; MODIFY_SIZE]>(self) }
    }

    fn decode(buffer: [u8; MODIFY_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; MODIFY_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for Modify {
    fn message_type(&self) -> MsgType {
        MsgType::Modify
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
