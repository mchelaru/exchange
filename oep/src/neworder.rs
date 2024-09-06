use std::error::Error;

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct NewOrder {
    pub client_order_id: u64,
    pub participant: u64,
    pub book_id: u64,
    pub quantity: u64,
    pub price: u64,
    pub order_type: u16,
    pub side: u8,
    pub gateway_id: u8,
    pub session_id: u32,
}

pub const NEWORDER_SIZE: usize = std::mem::size_of::<NewOrder>();

impl Decoder<NEWORDER_SIZE> for NewOrder {
    fn encode(self) -> [u8; NEWORDER_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; NEWORDER_SIZE]>(self) }
    }

    fn decode(buffer: [u8; NEWORDER_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; NEWORDER_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for NewOrder {
    fn message_type(&self) -> MsgType {
        MsgType::NewOrder
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
    use crate::{decoder::Decoder, neworder::NewOrder};

    #[test]
    fn decode() {
        let neworder_bytes = [
            66, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0,
            0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 66, 0, 1, 55, 66, 0, 0, 0,
        ];
        let boxed_target = NewOrder::decode(neworder_bytes);
        assert!(boxed_target.is_ok());
        let target = boxed_target.unwrap();

        assert_eq!(target.client_order_id as u64, 66);
        assert_eq!(target.participant as u64, 1);
        assert_eq!(target.book_id as u64, 2);
        assert_eq!(target.quantity as u64, 100);
        assert_eq!(target.side, 1);
        assert_eq!(target.order_type as u16, 66);
        assert_eq!(target.gateway_id, 55);
        assert_eq!(target.session_id as u32, 66);
    }

    #[test]
    fn test_encode_decode() {
        let new_order = NewOrder {
            client_order_id: 66,
            participant: 1,
            book_id: 2,
            quantity: 100,
            price: 1000,
            order_type: 66,
            side: 1,
            gateway_id: 55,
            session_id: 66,
        };

        let encoded = new_order.encode();

        let expected = [
            66, 0, 0, 0, 0, 0, 0, 0, // client_order_id
            1, 0, 0, 0, 0, 0, 0, 0, // participant
            2, 0, 0, 0, 0, 0, 0, 0, // book_id
            100, 0, 0, 0, 0, 0, 0, 0, // quantity
            232, 3, 0, 0, 0, 0, 0, 0, // price (1000)
            66, 0,  // order_type
            1,  // side
            55, // gateway_id
            66, 0, 0, 0, // session_id
        ];

        assert_eq!(encoded, expected);

        // Test that decoding the encoded data gives back the original struct
        let decoded = NewOrder::decode(encoded).unwrap();
        assert_eq!(
            decoded.client_order_id as u64,
            new_order.client_order_id as u64
        );
        assert_eq!(decoded.participant as u64, new_order.participant as u64);
        assert_eq!(decoded.book_id as u64, new_order.book_id as u64);
        assert_eq!(decoded.quantity as u64, new_order.quantity as u64);
        assert_eq!(decoded.price as u64, new_order.price as u64);
        assert_eq!(decoded.order_type as u16, new_order.order_type as u16);
        assert_eq!(decoded.side, new_order.side);
        assert_eq!(decoded.gateway_id, new_order.gateway_id);
        assert_eq!(decoded.session_id as u32, new_order.session_id as u32);
    }
}
