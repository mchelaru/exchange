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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modify_creation() {
        let modify = Modify {
            participant: 12345,
            order_id: 67890,
            book_id: 11111,
            quantity: 100,
            price: 1000,
            side: 1,
            gateway_id: 5,
            session_id: 33333,
        };

        assert_eq!(modify.participant as u64, 12345);
        assert_eq!(modify.order_id as u64, 67890);
        assert_eq!(modify.book_id as u64, 11111);
        assert_eq!(modify.quantity as u64, 100);
        assert_eq!(modify.price as u64, 1000);
        assert_eq!(modify.side, 1);
        assert_eq!(modify.gateway_id, 5);
        assert_eq!(modify.session_id as u32, 33333);
    }

    #[test]
    fn test_get_side() {
        let modify = Modify {
            participant: 12345,
            order_id: 67890,
            book_id: 11111,
            quantity: 100,
            price: 1000,
            side: 1,
            gateway_id: 5,
            session_id: 33333,
        };

        assert_eq!(modify.get_side(), 1);
    }

    #[test]
    fn test_encode_decode() {
        let original = Modify {
            participant: 12345,
            order_id: 67890,
            book_id: 11111,
            quantity: 100,
            price: 1000,
            side: 1,
            gateway_id: 5,
            session_id: 33333,
        };

        let encoded = original.encode();
        let decoded = Modify::decode(encoded).unwrap();

        assert_eq!(original.participant as u64, decoded.participant as u64);
        assert_eq!(original.order_id as u64, decoded.order_id as u64);
        assert_eq!(original.book_id as u64, decoded.book_id as u64);
        assert_eq!(original.quantity as u64, decoded.quantity as u64);
        assert_eq!(original.price as u64, decoded.price as u64);
        assert_eq!(original.side, decoded.side);
        assert_eq!(original.gateway_id, decoded.gateway_id);
        assert_eq!(original.session_id as u32, decoded.session_id as u32);
    }

    #[test]
    fn test_oep_message_trait() {
        let modify = Modify {
            participant: 12345,
            order_id: 67890,
            book_id: 11111,
            quantity: 100,
            price: 1000,
            side: 1,
            gateway_id: 5,
            session_id: 33333,
        };

        assert_eq!(modify.message_type(), MsgType::Modify);
        assert_eq!(modify.get_gateway_id(), 5);
        assert_eq!(modify.get_session_id(), 33333);
        assert_eq!(modify.get_participant(), 12345);
    }

    #[test]
    fn test_as_any() {
        let modify = Modify {
            participant: 12345,
            order_id: 67890,
            book_id: 11111,
            quantity: 100,
            price: 1000,
            side: 1,
            gateway_id: 5,
            session_id: 33333,
        };

        let any = modify.as_any();
        let downcast = any.downcast_ref::<Modify>();
        assert!(downcast.is_some());
        assert_eq!(downcast.unwrap().order_id as u64, 67890);
    }

    #[test]
    fn test_modify_size() {
        assert_eq!(MODIFY_SIZE, std::mem::size_of::<Modify>());
    }
}
