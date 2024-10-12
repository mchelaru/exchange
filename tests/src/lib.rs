mod test_exchange;

#[cfg(test)]
mod test {
    use std::io::{Read, Write};

    use oep::{
        neworder::{NewOrder, NEWORDER_SIZE},
        oep_message::OepMessage,
    };
    use order::{OrderState, OrderType, Side};

    use crate::test_exchange::test_exchange::TestExchange;

    #[test]
    fn connect_client_sockets() {
        let target = TestExchange::new();
        assert_eq!(
            9,
            target
                .client_socket
                .borrow_mut()
                .write(b"something")
                .expect("9")
        );

        let mut r: [u8; 10000] = [0; 10000];
        assert_eq!(
            9,
            target
                .gateway_client_socket
                .borrow_mut()
                .read(&mut r)
                .expect("9")
        );
    }

    #[test]
    fn send_login() {
        let mut target = TestExchange::new();
        let connection = target.login();
        assert!(connection.is_ok());
        // it replies back with login
        assert_eq!(
            8 + 144,
            target.client_socket.borrow().read_buffer.borrow().len()
        );
        // but it doesn't publish anything towards the matching engine
        assert_eq!(
            0,
            target
                .matching_engine_socket
                .borrow()
                .read_buffer
                .borrow()
                .len()
        );
    }

    /// New day order in an empty market
    #[test]
    fn process_new_day_order() {
        let mut target = TestExchange::new();
        let mut connection = target.login().unwrap();
        // "read" the login response
        target
            .client_socket
            .borrow()
            .read_buffer
            .borrow_mut()
            .clear();
        let input_order = NewOrder {
            client_order_id: 100,
            participant: 111,
            book_id: TestExchange::INSTRUMENT_ID,
            quantity: 100,
            price: 197,
            order_type: OrderType::Day.into(),
            side: Side::Bid.into(),
            gateway_id: 1,
            session_id: 2,
        };
        let boxed_message = Box::new(input_order.clone()) as Box<dyn OepMessage>;
        // first process the order at the gateway
        let send_order_result = target.send_order_to_gateway(&mut connection, &boxed_message);
        assert!(send_order_result.is_ok());

        // check if the matching engine input contains header + new order
        assert_eq!(
            4 + NEWORDER_SIZE,
            target
                .matching_engine_socket
                .borrow()
                .read_buffer
                .borrow()
                .len()
        );

        // and now process it at the matching engine
        let ereport = target.process_order_at_matching_engine();
        assert_eq!(ereport.state, OrderState::Inserted.into());
        let client_order_id = input_order.client_order_id;
        let submitted_order_id = ereport.submitted_order_id;
        assert_eq!(client_order_id, submitted_order_id);

        // test if the order was accepted by the market
        assert_eq!(1, target.market.generate_bids().len());

        // test if it published the new order on the feed
        let disseminator = target.disseminator.borrow();
        let feed_new_orders = disseminator.new_orders.borrow();
        assert_eq!(1, feed_new_orders.len());

        // check if what published on the feed matches the input
        let feed_order = &feed_new_orders[0];
        assert_eq!(input_order.get_participant(), feed_order.participant);
        assert_eq!(
            TestExchange::INSTRUMENT_ID,
            feed_order.instrument.borrow().get_id()
        );
        let input_quantity = input_order.quantity;
        assert_eq!(input_quantity, feed_order.quantity);
        let input_price = input_order.price;
        assert_eq!(input_price, feed_order.price);
        assert_eq!(input_order.side, feed_order.side.into());
        let input_type = input_order.order_type;
        assert_eq!(input_type, feed_order.order_type.into());
    }

    #[test]
    fn trade_against_standing_order() {
        let mut target = TestExchange::new();
        let mut connection = target.login().unwrap();
        let passive_order = NewOrder {
            client_order_id: 100,
            participant: 111,
            book_id: TestExchange::INSTRUMENT_ID,
            quantity: 100,
            price: 197,
            order_type: OrderType::Day.into(),
            side: Side::Bid.into(),
            gateway_id: 1,
            session_id: 2,
        };
        let boxed_message = Box::new(passive_order.clone()) as Box<dyn OepMessage>;
        // first process the order at the gateway
        let send_order_result = target.send_order_to_gateway(&mut connection, &boxed_message);
        assert!(send_order_result.is_ok());

        // and now process it at the matching engine
        let ereport = target.process_order_at_matching_engine();
        assert_eq!(ereport.state, OrderState::Inserted.into());

        // test if the order was accepted by the market
        assert_eq!(1, target.market.generate_bids().len());

        // more thorough checks are done in the process_new_day_order(), no need to repeat them here
        let aggressive_order = NewOrder {
            client_order_id: 100,
            participant: 111,
            book_id: TestExchange::INSTRUMENT_ID,
            quantity: 100,
            price: 197,
            order_type: OrderType::Day.into(),
            side: Side::Ask.into(),
            gateway_id: 1,
            session_id: 2,
        };
        let boxed_message = Box::new(aggressive_order.clone()) as Box<dyn OepMessage>;
        // first process the order at the gateway
        let send_order_result = target.send_order_to_gateway(&mut connection, &boxed_message);
        if send_order_result.is_err() {
            eprintln!("{:#?}", send_order_result);
        }
        assert!(send_order_result.is_ok());

        // and now process it at the matching engine
        let ereport = target.process_order_at_matching_engine();
        assert_eq!(ereport.state, OrderState::Traded.into());

        // test if the order was executed by the market
        assert_eq!(0, target.market.generate_bids().len());
    }
}
