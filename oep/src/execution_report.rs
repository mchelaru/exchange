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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_report_creation() {
        let er = ExecutionReport {
            participant: 12345,
            order_id: 67890,
            submitted_order_id: 11111,
            book: 22222,
            quantity: 100,
            price: 1000,
            flags: 0,
            side: 1,
            state: 2,
            session_id: 33333,
            gateway_id: 5,
        };

        assert_eq!(er.participant as u64, 12345);
        assert_eq!(er.order_id as u64, 67890);
        assert_eq!(er.submitted_order_id as u64, 11111);
        assert_eq!(er.book as u64, 22222);
        assert_eq!(er.quantity as u64, 100);
        assert_eq!(er.price as u64, 1000);
        assert_eq!(er.flags as u32, 0);
        assert_eq!(er.side, 1);
        assert_eq!(er.state, 2);
        assert_eq!(er.session_id as u32, 33333);
        assert_eq!(er.gateway_id, 5);
    }

    #[test]
    fn test_getter_methods() {
        let er = ExecutionReport {
            participant: 12345,
            order_id: 67890,
            submitted_order_id: 11111,
            book: 22222,
            quantity: 100,
            price: 1000,
            flags: 0,
            side: 1,
            state: 2,
            session_id: 33333,
            gateway_id: 5,
        };

        assert_eq!(er.get_book(), 22222);
        assert_eq!(er.get_price(), 1000);
        assert_eq!(er.get_submitted_order_id(), 11111);
        assert_eq!(er.get_order_id(), 67890);
        assert_eq!(er.get_quantity(), 100);
    }

    #[test]
    fn test_encode_decode() {
        let original = ExecutionReport {
            participant: 12345,
            order_id: 67890,
            submitted_order_id: 11111,
            book: 22222,
            quantity: 100,
            price: 1000,
            flags: 0,
            side: 1,
            state: 2,
            session_id: 33333,
            gateway_id: 5,
        };

        let encoded = original.encode();
        let decoded = ExecutionReport::decode(encoded).unwrap();

        assert_eq!(original.participant as u64, decoded.participant as u64);
        assert_eq!(original.order_id as u64, decoded.order_id as u64);
        assert_eq!(
            original.submitted_order_id as u64,
            decoded.submitted_order_id as u64
        );
        assert_eq!(original.book as u64, decoded.book as u64);
        assert_eq!(original.quantity as u64, decoded.quantity as u64);
        assert_eq!(original.price as u64, decoded.price as u64);
        assert_eq!(original.flags as u16, decoded.flags as u16);
        assert_eq!(original.side, decoded.side);
        assert_eq!(original.state, decoded.state);
        assert_eq!(original.session_id as u32, decoded.session_id as u32);
        assert_eq!(original.gateway_id, decoded.gateway_id);
    }

    #[test]
    fn test_oep_message_trait() {
        let er = ExecutionReport {
            participant: 12345,
            order_id: 67890,
            submitted_order_id: 11111,
            book: 22222,
            quantity: 100,
            price: 1000,
            flags: 0,
            side: 1,
            state: 2,
            session_id: 33333,
            gateway_id: 5,
        };

        assert_eq!(er.message_type(), MsgType::ExecutionReport);
        assert_eq!(er.get_gateway_id(), 5);
        assert_eq!(er.get_session_id(), 33333);
        assert_eq!(er.get_participant(), 12345);
    }

    #[test]
    fn test_as_any() {
        let er = ExecutionReport {
            participant: 12345,
            order_id: 67890,
            submitted_order_id: 11111,
            book: 22222,
            quantity: 100,
            price: 1000,
            flags: 0,
            side: 1,
            state: 2,
            session_id: 33333,
            gateway_id: 5,
        };

        let any = er.as_any();
        let downcast = any.downcast_ref::<ExecutionReport>();
        assert!(downcast.is_some());
        assert_eq!(downcast.unwrap().order_id as u64, 67890);
    }
}
