use std::error::Error;

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Cancel {
    pub participant: u64,
    pub order_id: u64,
    pub book_id: u64,
    pub side: u8,
    pub gateway_id: u8,
    pub session_id: u32,
}

impl Cancel {
    pub fn get_side(&self) -> u8 {
        self.side
    }
}

pub const CANCEL_SIZE: usize = std::mem::size_of::<Cancel>();

impl Decoder<CANCEL_SIZE> for Cancel {
    fn encode(self) -> [u8; CANCEL_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; CANCEL_SIZE]>(self) }
    }

    fn decode(buffer: [u8; CANCEL_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; CANCEL_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for Cancel {
    fn message_type(&self) -> MsgType {
        MsgType::Cancel
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        let cancel = Cancel {
            participant: 1234567890,
            order_id: 9876543210,
            book_id: 42,
            side: 1,
            gateway_id: 5,
            session_id: 987654,
        };

        let encoded = cancel.encode();

        let expected = [
            210, 2, 150, 73, 0, 0, 0, 0, // participant (1234567890)
            234, 22, 176, 76, 2, 0, 0, 0, // order_id (9876543210)
            42, 0, 0, 0, 0, 0, 0, 0, // book_id (42)
            1, // side
            5, // gateway_id
            6, 18, 15, 0, // session_id (987654)
        ];

        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_decode() {
        let buffer = [
            210, 2, 150, 73, 0, 0, 0, 0, // participant (1234567890)
            234, 22, 176, 76, 2, 0, 0, 0, // order_id (9876543210)
            42, 0, 0, 0, 0, 0, 0, 0, // book_id (42)
            1, // side
            5, // gateway_id
            6, 18, 15, 0, // session_id (987654)
        ];

        let decoded = Cancel::decode(buffer).unwrap();

        assert_eq!(decoded.participant as u64, 1234567890);
        assert_eq!(decoded.order_id as u64, 9876543210);
        assert_eq!(decoded.book_id as u64, 42);
        assert_eq!(decoded.side, 1);
        assert_eq!(decoded.gateway_id, 5);
        assert_eq!(decoded.session_id as u32, 987654);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = Cancel {
            participant: 1234567890,
            order_id: 9876543210,
            book_id: 42,
            side: 1,
            gateway_id: 5,
            session_id: 987654,
        };

        let encoded = original.encode();
        let decoded = Cancel::decode(encoded).unwrap();

        assert_eq!(decoded.participant as u64, original.participant as u64);
        assert_eq!(decoded.order_id as u64, original.order_id as u64);
        assert_eq!(decoded.book_id as u64, original.book_id as u64);
        assert_eq!(decoded.side, original.side);
        assert_eq!(decoded.gateway_id, original.gateway_id);
        assert_eq!(decoded.session_id as u32, original.session_id as u32);
    }

    #[test]
    fn test_oep_message_traits() {
        let cancel = Cancel {
            participant: 1234567890,
            order_id: 9876543210,
            book_id: 42,
            side: 1,
            gateway_id: 5,
            session_id: 987654,
        };

        assert_eq!(cancel.message_type(), MsgType::Cancel);
        assert_eq!(cancel.get_gateway_id(), 5);
        assert_eq!(cancel.get_session_id(), 987654);
        assert_eq!(cancel.get_participant(), 1234567890);
    }

    #[test]
    fn test_get_side() {
        let cancel = Cancel {
            participant: 1234567890,
            order_id: 9876543210,
            book_id: 42,
            side: 1,
            gateway_id: 5,
            session_id: 987654,
        };

        assert_eq!(cancel.get_side(), 1);
    }
}
