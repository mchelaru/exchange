#[cfg(test)]
pub(crate) mod test_exchange {
    use anyhow::Result;
    use disseminator::mockdisseminator::MockDisseminator;
    use instruments::instrument::{Instrument, InstrumentState, InstrumentType};
    use market::Market;
    use matching_engine::processor;
    use std::{
        cell::RefCell,
        io::{Read, Write},
        rc::Rc,
    };

    use gateway::messages::{receive_and_prepare_relay_message, ConnectedSession};
    use oep::{execution_report::ExecutionReport, login::Login, oep_message::OepMessage};
    use utils::network::MockSocket;
    pub(crate) struct TestExchange {
        pub client_socket: Rc<RefCell<MockSocket>>,
        pub gateway_client_socket: Rc<RefCell<MockSocket>>,
        pub gateway_sender: Rc<RefCell<MockSocket>>,
        pub matching_engine_socket: Rc<RefCell<MockSocket>>,
        pub disseminator: Rc<RefCell<MockDisseminator>>,
        #[allow(unused)]
        pub instrument: Rc<RefCell<Instrument>>,
        pub market: Market,
    }

    impl TestExchange {
        pub const INSTRUMENT_ID: u64 = 1000;

        /// Prepares an exchange that has a gateway, a matching engine and a disseminator,
        /// together with one market for an instrument in the trading state.
        /// Connects all the infrastructure components.
        pub(crate) fn new() -> Self {
            // prepare the market for our test instrument
            let instrument = Rc::new(RefCell::new(Instrument::new(
                Self::INSTRUMENT_ID,
                "TESTINST",
                InstrumentType::Share,
                InstrumentState::Trading,
                10,
                20,
            )));

            let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));

            let r = Self {
                client_socket: Rc::new(RefCell::new(MockSocket::new())),
                gateway_client_socket: Rc::new(RefCell::new(MockSocket::new())),
                gateway_sender: Rc::new(RefCell::new(MockSocket::new())),
                matching_engine_socket: Rc::new(RefCell::new(MockSocket::new())),
                disseminator: disseminator.clone(),
                instrument: instrument.clone(),
                market: Market::new(instrument, disseminator),
            };
            // first connect the client socket to the gateway input socket
            r.client_socket
                .borrow_mut()
                .connect_output(r.gateway_client_socket.clone());
            r.gateway_client_socket
                .borrow_mut()
                .connect_output(r.client_socket.clone());

            // connect gateway output to the matching engine input
            r.gateway_sender
                .borrow_mut()
                .connect_output(r.matching_engine_socket.clone());

            return r;
        }

        pub(crate) fn login(&mut self) -> Result<ConnectedSession<MockSocket>> {
            let mut mockdb = dbhook::factory::build("mock");
            let mut connection = ConnectedSession::new(self.gateway_client_socket.clone());
            let login_message = Box::new(Login::new(1, 1, 1, "test")) as Box<dyn OepMessage>;
            let r = receive_and_prepare_relay_message(&mut mockdb, &mut connection, &login_message);
            assert!(r.is_ok());
            assert_eq!(0, connection.response_buffer.len()); // nothing is sent further to the matching engine

            return Ok(connection);
        }

        /// sends an order to the gateway input
        /// and relays it to the matching engine
        pub(crate) fn send_order_to_gateway(
            &mut self,
            connection: &mut ConnectedSession<MockSocket>,
            boxed_message: &Box<dyn OepMessage>,
        ) -> Result<u64> {
            let mut mockdb = dbhook::factory::build("mock");
            let result = receive_and_prepare_relay_message(&mut mockdb, connection, &boxed_message);

            // check if we should relay anything to the matching engine
            if connection.response_buffer.len() > 0 {
                self.gateway_sender
                    .borrow_mut()
                    .write(&connection.response_buffer)?;
                connection.response_buffer.clear();
            }

            result
        }

        pub(crate) fn process_order_at_matching_engine(&mut self) -> ExecutionReport {
            let mut buf = [0; 2000];
            let r = self.matching_engine_socket.borrow_mut().read(&mut buf);
            assert!(r.is_ok());
            let r = r.unwrap();
            assert!(r > 4);
            let (msg, book_id) = processor::decode_message(&buf[0..r]).unwrap();
            assert_eq!(Self::INSTRUMENT_ID, book_id);
            processor::process_message(&mut self.market, msg)
        }
    }
}
