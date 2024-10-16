use std::cell::RefCell;

use oep::trade::Trade;
use order::Order;

use crate::disseminator::Disseminator;
use instruments::instrument::Instrument;

/// MockDisseminator used for the market unit tests
#[derive(Debug)]
pub struct MockDisseminator {
    pub cancels: RefCell<Vec<Order>>,
    pub new_orders: RefCell<Vec<Order>>,
    pub modifies: RefCell<Vec<Order>>,
    pub trades: RefCell<Vec<Trade>>,
    pub instrument_info: RefCell<Vec<Instrument>>,
    // not really "market orders" but orders that can be used to reconstruct a market
    pub market_orders: RefCell<Vec<Order>>,
}

impl MockDisseminator {
    pub fn new() -> Self {
        Self {
            cancels: RefCell::new(vec![]),
            new_orders: RefCell::new(vec![]),
            modifies: RefCell::new(vec![]),
            trades: RefCell::new(vec![]),
            instrument_info: RefCell::new(vec![]),
            market_orders: RefCell::new(vec![]),
        }
    }
}

impl Disseminator for MockDisseminator {
    fn send_cancel_order(&self, order: &order::Order) -> Result<usize, std::io::Error> {
        self.cancels.borrow_mut().push(order.clone());
        Ok(1)
    }

    fn send_new_order(&self, order: &order::Order) -> Result<usize, std::io::Error> {
        self.new_orders.borrow_mut().push(order.clone());
        Ok(1)
    }

    fn send_trade(&self, trade: &Trade) -> Result<usize, std::io::Error> {
        self.trades.borrow_mut().push(trade.clone());
        Ok(1)
    }

    fn send_modify_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        self.modifies.borrow_mut().push(order.clone());
        Ok(1)
    }

    fn send_instrument_info(&self, instrument: &Instrument) -> Result<usize, std::io::Error> {
        self.instrument_info.borrow_mut().push(instrument.clone());
        Ok(1)
    }

    fn send_market_order(&self, order: &Order) -> Result<usize, std::io::Error> {
        self.market_orders.borrow_mut().push(order.clone());
        Ok(1)
    }
}
