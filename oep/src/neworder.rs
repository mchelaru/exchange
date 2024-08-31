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

        let order_id = target.client_order_id;
        let participant = target.participant;
        let book = target.book_id;
        let quantity = target.quantity;
        let flags = target.order_type;
        let gateway_id = target.gateway_id;
        let session_id = target.session_id;

        assert_eq!(order_id, 66);
        assert_eq!(participant, 1);
        assert_eq!(book, 2);
        assert_eq!(quantity, 100);
        assert_eq!(target.side, 1);
        assert_eq!(flags, 66);
        assert_eq!(gateway_id, 55);
        assert_eq!(session_id, 66);
    }
}
