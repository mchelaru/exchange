use instruments::instrument::Instrument;
/// I do grotesque things in this file just for the sake of testing
///
///
use oep::{cancel::Cancel, decoder::Decoder, modify::Modify, neworder::NewOrder, trade::Trade};
use order::Order;
#[cfg(not(test))]
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::cell::Cell;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(not(test))]
use std::net::{Ipv4Addr, SocketAddrV4};

use crate::disseminator::Disseminator;

#[cfg(not(test))]
#[derive(Debug)]
pub struct MBOOepDisseminator {
    socket: Socket,
    seq: Cell<u64>,
}

#[cfg(test)]
#[derive(Debug)]
struct MockSocket {
    pub buffer: RefCell<Vec<u8>>,
}

#[cfg(test)]
impl MockSocket {
    pub fn send(&self, bytes: &[u8]) -> Result<usize, std::io::Error> {
        self.buffer.borrow_mut().append(&mut bytes.to_vec());
        Ok(bytes.len())
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct MBOOepDisseminator {
    socket: MockSocket,
    seq: Cell<u64>,
}

impl MBOOepDisseminator {
    #[cfg(not(test))]
    pub fn new(addr: &str, port: u16) -> Self {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
        socket
            .connect(&SockAddr::from(SocketAddrV4::new(
                addr.parse::<Ipv4Addr>().unwrap(),
                port,
            )))
            .expect("Error connecting the disseminator");
        socket
            .set_multicast_loop_v4(true)
            .expect("set_multicast_loop_v4");
        Self {
            socket: socket,
            seq: Cell::new(0),
        }
    }

    fn send(&self, bytes: &[u8]) -> Result<usize, std::io::Error> {
        let old_seq = self.seq.get();
        self.seq.set(old_seq + 1);
        self.socket
            .send([&old_seq.to_le_bytes(), bytes].concat().as_slice())
    }

    fn send_with_header(&self, header_bytes: &[u8], bytes: &[u8]) -> Result<usize, std::io::Error> {
        self.send([header_bytes, bytes].concat().as_slice())
    }
}

impl Disseminator for MBOOepDisseminator {
    fn send_cancel_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        let cancel_order_header = [6];
        let m = Cancel {
            participant: order.participant,
            order_id: order.get_id(),
            book_id: order.instrument.borrow().get_id(),
            gateway_id: 0,
            session_id: 0,
            side: order.side.into(),
        };
        self.send_with_header(&cancel_order_header, &m.encode())
    }

    fn send_new_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        let new_order_header = [4];
        let m = NewOrder {
            client_order_id: order.get_id(),
            participant: order.participant,
            book_id: order.instrument.borrow().get_id(),
            quantity: order.quantity,
            price: order.price,
            order_type: order.order_type.into(),
            side: order.side.into(),
            gateway_id: 0,
            session_id: 0,
        };
        self.send_with_header(&new_order_header, &m.encode())
    }

    fn send_modify_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        let modify_header = [5];
        let m = Modify {
            participant: order.participant,
            order_id: order.get_id(),
            book_id: order.instrument.borrow().get_id(),
            quantity: order.quantity,
            price: order.price,
            gateway_id: 0,
            session_id: 0,
            side: order.side.into(),
        };
        self.send_with_header(&modify_header, &m.encode())
    }

    fn send_trade(&self, trade: &Trade) -> Result<usize, std::io::Error> {
        let trade_header = [3];
        self.send_with_header(&trade_header, &trade.encode())
    }

    fn send_instrument_info(&self, instrument: &Instrument) -> Result<usize, std::io::Error> {
        let instrument_header = [1];
        self.send_with_header(&instrument_header, &instrument.encode())
    }

    fn send_market_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        let market_header = [2];
        let m = NewOrder {
            client_order_id: order.get_id(),
            participant: order.participant,
            book_id: order.instrument.borrow().get_id(),
            quantity: order.quantity,
            price: order.price,
            order_type: order.order_type.into(),
            side: order.side.into(),
            gateway_id: 0,
            session_id: 0,
        };
        self.send_with_header(&market_header, &m.encode())
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use instruments::instrument::Instrument;
    use oep::decoder::Decoder;
    use order::{Order, Side};

    use crate::disseminator::Disseminator;

    use super::MBOOepDisseminator;

    #[test]
    pub fn send_new_order() {
        const BOOK_ID: u64 = 444;
        let instrument =
            Instrument::new_fast(BOOK_ID, instruments::instrument::InstrumentType::Share);
        let order = Order::new(
            1001,
            Rc::new(RefCell::new(instrument)),
            123,
            100,
            order::Side::Bid,
            order::OrderType::Day,
            100,
            1001,
        );
        let target = MBOOepDisseminator {
            socket: super::MockSocket {
                buffer: RefCell::new(vec![]),
            },
            seq: Cell::new(0),
        };

        let v = target.send_new_order(&order);
        assert!(v.is_ok());

        let buf = oep::neworder::NewOrder {
            client_order_id: order.get_id(),
            participant: order.participant,
            book_id: BOOK_ID,
            quantity: order.quantity,
            price: order.price,
            order_type: order.order_type.into(),
            side: order.side.into(),
            gateway_id: 0,
            session_id: 0,
        }
        .encode();
        let buf = [[0, 0, 0, 0, 0, 0, 0, 0, 4].as_slice(), buf.as_slice()].concat();
        assert_eq!(buf, target.socket.buffer.borrow().clone());
        assert!(buf.len() > 20);
    }

    #[test]
    pub fn send_cancel_order() {
        const BOOK_ID: u64 = 444;
        let instrument =
            Instrument::new_fast(BOOK_ID, instruments::instrument::InstrumentType::Share);
        let order = Order::new(
            1001,
            Rc::new(RefCell::new(instrument)),
            123,
            100,
            order::Side::Bid,
            order::OrderType::Day,
            100,
            10001,
        );
        let target = MBOOepDisseminator {
            socket: super::MockSocket {
                buffer: RefCell::new(vec![]),
            },
            seq: Cell::new(0),
        };

        let v = target.send_cancel_order(&order);
        assert!(v.is_ok());

        let buf = oep::cancel::Cancel {
            participant: order.participant,
            order_id: order.get_id(),
            book_id: order.instrument.borrow().get_id(),
            side: Side::Bid.into(),
            gateway_id: 0,
            session_id: 0,
        }
        .encode();
        let buf = [[0, 0, 0, 0, 0, 0, 0, 0, 6].as_slice(), buf.as_slice()].concat();
        assert_eq!(buf, target.socket.buffer.borrow().clone());
        assert!(buf.len() > 20);
    }

    #[test]
    pub fn send_modify_order() {
        const BOOK_ID: u64 = 444;
        let instrument =
            Instrument::new_fast(BOOK_ID, instruments::instrument::InstrumentType::Share);
        let order = Order::new(
            1001,
            Rc::new(RefCell::new(instrument)),
            123,
            100,
            order::Side::Bid,
            order::OrderType::Day,
            100,
            1001,
        );
        let target = MBOOepDisseminator {
            socket: super::MockSocket {
                buffer: RefCell::new(vec![]),
            },
            seq: Cell::new(0),
        };

        let v = target.send_modify_order(&order);
        assert!(v.is_ok());

        let buf = oep::modify::Modify {
            participant: order.participant,
            order_id: order.get_id(),
            book_id: order.instrument.borrow().get_id(),
            side: Side::Bid.into(),
            gateway_id: 0,
            session_id: 0,
            quantity: order.quantity,
            price: order.price,
        }
        .encode();
        let buf = [[0, 0, 0, 0, 0, 0, 0, 0, 5].as_slice(), buf.as_slice()].concat();
        assert_eq!(buf, target.socket.buffer.borrow().clone());
        assert!(buf.len() > 20);
    }

    #[test]
    pub fn increments_sequence() {
        const BOOK_ID: u64 = 444;
        let instrument =
            Instrument::new_fast(BOOK_ID, instruments::instrument::InstrumentType::Share);
        let order = Order::new(
            1001,
            Rc::new(RefCell::new(instrument)),
            123,
            100,
            order::Side::Bid,
            order::OrderType::Day,
            100,
            1001,
        );
        let target = MBOOepDisseminator {
            socket: super::MockSocket {
                buffer: RefCell::new(vec![]),
            },
            seq: Cell::new(0),
        };

        for s in 0..10 {
            assert!(target.send_modify_order(&order).is_ok());
            let v = target.socket.buffer.borrow().clone();
            let seq = u64::from_le_bytes(v[0..8].try_into().expect("cannot convert"));
            assert_eq!(s, seq);
            target.socket.buffer.borrow_mut().clear();
        }
    }

    #[test]
    fn send_instrument() {
        let instrument = Instrument::new(
            400,
            "XYZ",
            instruments::instrument::InstrumentType::Share,
            instruments::instrument::InstrumentState::Trading,
            10,
            20,
        );

        let target = MBOOepDisseminator {
            socket: super::MockSocket {
                buffer: RefCell::new(vec![]),
            },
            seq: Cell::new(0),
        };
        assert!(target.send_instrument_info(&instrument).is_ok());
        assert_eq!((8 + 1 + (12 + 3)) * 1, target.socket.buffer.borrow().len());

        let decoded_instrument = Instrument::decode(
            target.socket.buffer.borrow().clone()[9..24]
                .try_into()
                .expect("cannot convert"),
        );
        assert_eq!(400, decoded_instrument.get_id());
        assert_eq!(
            instruments::instrument::InstrumentType::Share,
            decoded_instrument.get_type()
        );
        assert_eq!(
            instruments::instrument::InstrumentState::Trading,
            decoded_instrument.get_state()
        );
        assert_eq!(10, decoded_instrument.get_percentage_bands());
        assert_eq!(20, decoded_instrument.get_percentage_variation_allowed());
        assert_eq!("XYZ", decoded_instrument.get_name());
    }
}
