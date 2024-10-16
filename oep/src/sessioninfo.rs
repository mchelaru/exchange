use crate::{
    oep_message::{MsgType, OepMessage},
    Decoder,
};
use std::error::Error;

/// not a real OEP message, but instead it's sent
/// by the gateway to the matching engine when a session disconnects

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct SessionInfo {
    participant: u64,
    session_id: u32,
    gateway_id: u8,
}

impl SessionInfo {
    pub fn new(participant: u64, session_id: u32, gateway_id: u8) -> Self {
        Self {
            participant: participant,
            session_id: session_id,
            gateway_id: gateway_id,
        }
    }
}

pub const SESSIONINFO_SIZE: usize = std::mem::size_of::<SessionInfo>();

impl Decoder<SESSIONINFO_SIZE> for SessionInfo {
    fn encode(self) -> [u8; SESSIONINFO_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; SESSIONINFO_SIZE]>(self) }
    }

    fn decode(buffer: [u8; SESSIONINFO_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; SESSIONINFO_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for SessionInfo {
    fn message_type(&self) -> MsgType {
        MsgType::SessionNotification
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
