#[cfg(test)]
mod tests {
    use crate::{neworder::NewOrder, oep_decode, oep_message::MsgType};

    #[test]
    fn decode_new_order() {
        let new_order_buffer = [
            1, 0, 0, 0, 20, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0,
            0, 0, 0, 0, 101, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 66, 0, 1, 55, 22, 0, 0,
            0,
        ];
        let msg = oep_decode(&new_order_buffer);
        if msg.is_err() {
            let x = msg.as_ref().err().unwrap();
            println!("Error in decoding: {}", x.to_string());
        }
        assert!(msg.is_ok());
        match msg {
            Ok(boxed_msg) => {
                let msg = boxed_msg.as_ref();
                assert_eq!(msg.message_type(), MsgType::NewOrder);
                let new_order: &NewOrder = msg
                    .as_any()
                    .downcast_ref::<NewOrder>()
                    .expect("Bad pointer conversion");
                let order_id = new_order.client_order_id;
                let participant = new_order.participant;
                let book = new_order.book_id;
                let quantity = new_order.quantity;
                let flags = new_order.order_type;
                let gateway_id = new_order.gateway_id;
                let session_id = new_order.session_id;
                let price = new_order.price;

                assert_eq!(order_id, 50);
                assert_eq!(participant, 1);
                assert_eq!(book, 2);
                assert_eq!(quantity, 101);
                assert_eq!(price, 100);
                assert_eq!(new_order.side, 1);
                assert_eq!(flags, 66);
                assert_eq!(gateway_id, 55);
                assert_eq!(session_id, 22);
            }
            Err(_) => todo!(), // already matched up
        }
    }

    #[test]
    fn short_header() {
        let new_order_buffer = [1, 0, 0, 0, 20];
        let msg = oep_decode(&new_order_buffer);
        assert!(msg.is_err());
    }

    #[test]
    fn short_message() {
        let new_order_buffer = [1, 0, 0, 0, 20, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0];
        let msg = oep_decode(&new_order_buffer);
        assert!(msg.is_err());
    }

    #[test]
    fn too_short_until_complete() {
        let new_order_buffer = [
            1, 0, 0, 0, 20, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0,
            0, 0, 0, 0, 101, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 66, 0, 1, 55, 22, 0, 0,
            0,
        ];
        for i in 0..new_order_buffer.len() {
            let msg = oep_decode(&new_order_buffer[..i]);
            assert!(msg.is_err());
        }
        let msg = oep_decode(&new_order_buffer);
        if msg.is_err() {
            let x = msg.as_ref().err().unwrap();
            println!("Error in decoding: {}", x.to_string());
        }
        assert!(msg.is_ok());
        match msg {
            Ok(boxed_msg) => {
                let msg = boxed_msg.as_ref();
                assert_eq!(msg.message_type(), MsgType::NewOrder);
                let new_order: &NewOrder = msg
                    .as_any()
                    .downcast_ref::<NewOrder>()
                    .expect("Bad pointer conversion");
                let order_id = new_order.client_order_id;
                let participant = new_order.participant;
                let book = new_order.book_id;
                let quantity = new_order.quantity;
                let price = new_order.price;
                let flags = new_order.order_type;
                let gateway_id = new_order.gateway_id;
                let session_id = new_order.session_id;

                assert_eq!(order_id, 60);
                assert_eq!(participant, 1);
                assert_eq!(book, 2);
                assert_eq!(quantity, 101);
                assert_eq!(price, 100);
                assert_eq!(new_order.side, 1);
                assert_eq!(flags, 66);
                assert_eq!(gateway_id, 55);
                assert_eq!(session_id, 22);
            }
            Err(_) => todo!(), // already matched up
        }
    }
}
