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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // this test is quite dumb for now but maybe
    // at some point I will change the instantion way
    fn test_trade_creation() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };

        assert_eq!(trade.bid_order_id as u64, 12345);
        assert_eq!(trade.ask_order_id as u64, 67890);
        assert_eq!(trade.price as u64, 1000);
        assert_eq!(trade.quantity as u64, 100);
    }

    #[test]
    fn test_encode_decode() {
        let original = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };

        let encoded = original.encode();
        let decoded = Trade::decode(encoded).unwrap();

        assert_eq!(original.bid_order_id as u64, decoded.bid_order_id as u64);
        assert_eq!(original.ask_order_id as u64, decoded.ask_order_id as u64);
        assert_eq!(original.price as u64, decoded.price as u64);
        assert_eq!(original.quantity as u64, decoded.quantity as u64);
    }

    #[test]
    fn test_oep_message_trait() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };

        assert_eq!(trade.message_type(), MsgType::Trade);
    }

    #[test]
    fn test_as_any() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };

        let any = trade.as_any();
        let downcast = any.downcast_ref::<Trade>();
        assert!(downcast.is_some());
        assert_eq!(downcast.unwrap().bid_order_id as u64, 12345);
    }

    #[test]
    fn test_trade_size() {
        assert_eq!(TRADE_SIZE, std::mem::size_of::<Trade>());
    }

    #[test]
    #[should_panic(expected = "No gateway in a trade message")]
    fn test_get_gateway_id_panic() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };
        trade.get_gateway_id();
    }

    #[test]
    #[should_panic(expected = "No session in a trade message")]
    fn test_get_session_id_panic() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };
        trade.get_session_id();
    }

    #[test]
    #[should_panic(expected = "No participant in a trade message")]
    fn test_get_participant_panic() {
        let trade = Trade {
            bid_order_id: 12345,
            ask_order_id: 67890,
            price: 1000,
            quantity: 100,
        };
        trade.get_participant();
    }
}
