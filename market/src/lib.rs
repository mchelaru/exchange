use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use disseminator::disseminator::Disseminator;
use instruments::instrument::{Instrument, InstrumentState};
use order::{Order, OrderState, OrderType, Side};

#[derive(Debug, Clone)]
pub struct Market {
    instrument: Rc<RefCell<Instrument>>,
    bids: VecDeque<Order>,
    asks: VecDeque<Order>,
    order_id: u64,
    disseminator: Rc<RefCell<dyn Disseminator>>,

    bids_ops: u32,
    asks_ops: u32,
}

const REARRANGE_THRESHOLD: u32 = 10000;

/// Structure that holds a market for a certain instrument
///
/// In order to manipulate the market, user has three main functions:
/// @add_order, @modify_order and @cancel_order
/// Arguments should be an order structure as defined in the order library
///
/// Other notable functions:
/// @publish_snapshot -> publishes instrument and market snapshot on the disseminator socket
/// @get_state -> returns the instrument state that is implicitely assumed to also be the market state
impl Market {
    /// Create a market for a certain instrument and attaches a feed disseminator
    pub fn new(
        instrument: Rc<RefCell<Instrument>>,
        disseminator: Rc<RefCell<dyn Disseminator>>,
    ) -> Self {
        Self {
            instrument: instrument,
            bids: VecDeque::new(),
            asks: VecDeque::new(),
            order_id: 0,
            disseminator: disseminator.clone(),
            bids_ops: 0,
            asks_ops: 0,
        }
    }

    /// Close the market and cancel all the orders
    pub fn close(&mut self) {
        self.instrument
            .borrow_mut()
            .set_state(InstrumentState::Closed);
        let mut iter = self.bids.iter().chain(self.asks.iter()).into_iter();
        while let Some(o) = iter.next() {
            self.publish_cancel_order(&o);
        }
        self.bids.clear();
        self.asks.clear();
    }

    fn publish_cancel_order(&self, o: &Order) {
        self.disseminator.borrow().send_cancel_order(o).unwrap();
    }

    fn publish_new_order(&self, o: &Order) {
        self.disseminator.borrow().send_new_order(o).unwrap();
    }

    fn publish_modified_order(&self, o: &Order) {
        self.disseminator.borrow().send_modify_order(o).unwrap();
    }

    fn publish_trade(&self, trade: &oep::trade::Trade) {
        self.disseminator.borrow().send_trade(trade).unwrap();
    }

    pub fn add_order(&mut self, mut o: Order) -> (OrderState, u64) {
        assert_eq!(
            self.instrument.borrow().get_id(),
            o.instrument.borrow().get_id()
        );
        if InstrumentState::Closed == self.instrument.borrow().get_state() {
            return (OrderState::Rejected, 0);
        }

        self.order_id += 1;
        o.set_id(self.order_id); // FIXME: who is using this, since the value is not returned?

        if o.quantity == 0 || (o.price == 0 && o.order_type != OrderType::Market) {
            return (OrderState::Rejected, 0);
        }

        // Check out of bands
        if o.order_type != OrderType::Market && self.bids.len() > 0 && self.asks.len() > 0 {
            let midpoint =
                (self.bids.front().unwrap().price + self.asks.front().unwrap().price) / 2;
            if o.price
                < midpoint * (100 - self.instrument.borrow().get_percentage_bands() as u64) / 100
                || o.price
                    > midpoint * (100 + self.instrument.borrow().get_percentage_bands() as u64)
                        / 100
            {
                return (OrderState::Rejected, 0);
            }
        }

        if self.bids_ops > REARRANGE_THRESHOLD {
            self.bids.make_contiguous();
            self.bids_ops = 0;
        } else if self.asks_ops > REARRANGE_THRESHOLD {
            self.asks.make_contiguous();
            self.asks_ops = 0;
        }

        macro_rules! trade_and_add {
            ($list:expr, $comp:ident, $order:expr) => {{
                let mut trades = 0;
                while $order.quantity > 0
                    && $list.len() > 0
                    && ($order.price.$comp(&$list.front().unwrap().price)
                        || $order.order_type == OrderType::Market)
                {
                    // trade
                    let trade_volume =
                        std::cmp::min($list.front().unwrap().quantity, $order.quantity);
                    $order.quantity -= trade_volume;
                    // passive order massaging
                    let mut p = $list.pop_front().unwrap();
                    p.quantity -= trade_volume;
                    if p.quantity > 0 {
                        $list.push_front(p.clone());
                    }
                    // publish it
                    self.publish_trade(&oep::trade::Trade {
                        bid_order_id: if $order.side == Side::Bid {
                            $order.get_id()
                        } else {
                            p.get_id()
                        },
                        ask_order_id: if $order.side == Side::Ask {
                            $order.get_id()
                        } else {
                            p.get_id()
                        },
                        price: p.price,
                        quantity: trade_volume,
                    });
                    trades += 1;
                }
                if $order.quantity == 0 {
                    // aggressor fully traded
                    return (OrderState::Traded, $order.get_id());
                }
                match $order.order_type {
                    OrderType::FillAndKill | OrderType::FillOrKill => {
                        return (OrderState::Cancelled, $order.get_id())
                    }
                    OrderType::Market => match trades {
                        0 => return (OrderState::Cancelled, $order.get_id()),
                        _ => return (OrderState::Traded, $order.get_id()),
                    },
                    _ => {
                        self.insert_into_right_position(&$order);
                        if trades > 0 {
                            return (OrderState::PartiallyTraded, $order.get_id());
                        } else {
                            self.publish_new_order(&$order);
                            return (OrderState::Inserted, $order.get_id());
                        }
                    }
                }
            }};
        }

        match o.side {
            Side::Bid => {
                self.bids_ops += 1;
                trade_and_add!(self.asks, ge, o)
            }
            Side::Ask => {
                self.asks_ops += 1;
                trade_and_add!(self.bids, le, o)
            }
        }
    }

    fn insert_into_right_position(&mut self, o: &Order) {
        macro_rules! fit_into_position {
            ($list:expr, $comp:ident, $order:expr) => {{
                let mut pos = 0;
                for b in &$list {
                    if b.price.$comp(&$order.price) {
                        break;
                    }
                    pos += 1;
                }
                $list.insert(pos, $order.clone());
            }};
        }
        match o.side {
            Side::Bid => fit_into_position!(self.bids, lt, o),
            Side::Ask => fit_into_position!(self.asks, gt, o),
        }
    }

    pub fn modify_order(&mut self, o: Order) -> (OrderState, u64) {
        assert_eq!(
            o.instrument.borrow().get_id(),
            self.instrument.borrow().get_id()
        );

        // run some basic checks
        if o.quantity == 0 {
            return (OrderState::Rejected, 0);
        }

        macro_rules! remove_and_add {
            ($side: expr) => {
                match $side.iter().position(|x| {
                    x.participant == o.participant
                        && x.gateway_id == o.gateway_id
                        && x.session_id == o.session_id
                        && x.get_id() == o.get_id()
                        && x.order_type == o.order_type
                }) {
                    Some(index) => {
                        if o.price == $side[index].price {
                            // change the quantity only
                            $side[index].quantity = o.quantity;
                            self.publish_modified_order(&$side[index]);
                            (OrderState::Modified, $side[index].get_id())
                        } else {
                            self.publish_cancel_order(&$side[index]);
                            $side.remove(index);
                            self.add_order(o)
                        }
                    }
                    None => return (OrderState::Rejected, 0),
                }
            };
        }

        match o.side {
            Side::Bid => remove_and_add!(self.bids),
            Side::Ask => remove_and_add!(self.asks),
        }
    }

    pub fn cancel_order(&mut self, o: &Order) -> OrderState {
        assert_eq!(
            o.instrument.borrow().get_id(),
            self.instrument.borrow().get_id()
        );
        macro_rules! remove {
            ($side: expr) => {
                match $side.iter().position(|x| {
                    x.participant == o.participant
                        && x.gateway_id == o.gateway_id
                        && x.session_id == o.session_id
                        && x.get_id() == o.get_id()
                }) {
                    Some(index) => {
                        if let Some(canceled_order) = $side.remove(index) {
                            self.publish_cancel_order(&canceled_order);
                        }
                        OrderState::Cancelled
                    }
                    None => OrderState::Rejected,
                }
            };
        }
        match o.side {
            Side::Bid => {
                self.bids_ops += 1;
                remove!(self.bids)
            }
            Side::Ask => {
                self.asks_ops += 1;
                remove!(self.asks)
            }
        }
    }

    #[must_use]
    /// cancels all the standing orders for a certain (participant, gateway, session) tuple
    ///
    /// Returns: a vector of tuples (order_id, book_id, side)
    pub fn cancel_all_orders_for_session(
        &mut self,
        participant: u64,
        gateway_id: u8,
        session_id: u32,
    ) -> Vec<(u64, u64, Side)> {
        let bid_matches: Vec<Order> = self
            .bids
            .iter()
            .filter(|&o| {
                o.participant == participant
                    && o.gateway_id == gateway_id
                    && o.session_id == session_id
            })
            .map(|o| o.clone())
            .collect();
        let ask_matches: Vec<Order> = self
            .asks
            .iter()
            .filter(|&o| {
                o.participant == participant
                    && o.gateway_id == gateway_id
                    && o.session_id == session_id
            })
            .map(|o| o.clone())
            .collect();

        for o in bid_matches.iter().chain(ask_matches.iter()) {
            self.cancel_order(o);
        }

        self.bids_ops += bid_matches.len() as u32;
        self.asks_ops += ask_matches.len() as u32;

        // return both matching bids and asks
        bid_matches
            .iter()
            .chain(ask_matches.iter())
            .map(|o| (o.get_id(), o.instrument.borrow().get_id(), o.side.into()))
            .collect()
    }

    pub fn generate_bids(&self) -> Vec<&Order> {
        self.bids.iter().collect()
    }

    pub fn generate_asks(&self) -> Vec<&Order> {
        self.asks.iter().collect()
    }

    #[cfg(test)]
    pub(crate) fn set_state_trading(&mut self) {
        self.instrument
            .borrow_mut()
            .set_state(InstrumentState::Trading);
    }

    pub fn get_instrument(&self) -> Rc<RefCell<Instrument>> {
        self.instrument.clone()
    }

    pub fn get_state(&self) -> InstrumentState {
        self.instrument.borrow().get_state()
    }

    pub fn get_order_id(&self) -> u64 {
        self.order_id
    }

    /// Publishes the state of the registered instrument and the snapshot
    /// of the market
    pub fn publish_snapshot(&self) -> Result<usize, std::io::Error> {
        let mut result = self
            .disseminator
            .borrow()
            .send_instrument_info(&self.instrument.borrow())?;

        for o in self
            .generate_bids()
            .iter()
            .chain(self.generate_asks().iter())
        {
            result += self.disseminator.borrow().send_market_order(o)?;
        }

        Ok(result)
    }

    pub fn instrument_updated(&self) {}
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use disseminator::mockdisseminator::MockDisseminator;
    use instruments::instrument::{Instrument, InstrumentState, InstrumentType};

    use order::{Order, OrderState, OrderType, Side};

    use super::Market;

    #[test]
    fn order_insert() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o);
        assert_eq!(r.0, OrderState::Inserted);

        let bids = target.generate_bids();
        assert_eq!(1, bids.len());
        assert_eq!(1000, bids[0].participant);
        assert_eq!(123, bids[0].price);
        assert_eq!(100, bids[0].quantity);
        assert_eq!(Side::Bid, bids[0].side);
        assert_eq!(OrderType::Day, bids[0].order_type);

        assert_eq!(500, bids[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            bids[0].instrument.borrow().get_type()
        );
    }

    #[test]
    fn ioc_cancelled() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::FillOrKill,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o);
        assert_eq!(r.0, OrderState::Cancelled); // nothing to match against
        assert_eq!(0, target.generate_bids().len());
        assert_eq!(0, target.generate_asks().len());
    }

    #[test]
    fn new_order_zero_quantity_rejected() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            123,
            0,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o);
        assert_eq!(r.0, OrderState::Rejected);

        let bids = target.generate_bids();
        assert_eq!(0, bids.len());
    }

    #[test]
    fn cross_completely() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o_passive = Order::new(
            1000,
            i.clone(),
            123,
            400,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_aggressive = Order::new(
            1001,
            i.clone(),
            123,
            100,
            Side::Ask,
            OrderType::Day,
            101,
            2001,
        );

        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o_passive).0);
        let id1 = target.get_order_id();

        assert_eq!(OrderState::Traded, target.add_order(o_aggressive).0);
        let id2 = target.get_order_id();

        assert_eq!(0, target.generate_asks().len());

        let bids = target.generate_bids();
        assert_eq!(1, bids.len());
        assert_eq!(1000, bids[0].participant);
        assert_eq!(123, bids[0].price);
        assert_eq!(300, bids[0].quantity);
        assert_eq!(Side::Bid, bids[0].side);
        assert_eq!(OrderType::Day, bids[0].order_type);

        assert_eq!(500, bids[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            bids[0].instrument.borrow().get_type()
        );

        // make sure we're publishing the trade
        assert_eq!(1, disseminator.borrow().trades.borrow().len());
        let binding = disseminator.borrow();
        let trades = binding.trades.borrow();
        let trade = trades.get(0).unwrap();
        let bid_id = trade.bid_order_id;
        let ask_id = trade.ask_order_id;
        let quantity = trade.quantity;
        let price = trade.price;

        assert_eq!(id1, bid_id);
        assert_eq!(id2, ask_id);
        assert_eq!(100, quantity);
        assert_eq!(123, price);
    }

    #[test]
    fn cross_partially_and_post() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o_passive = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_aggressive = Order::new(
            1001,
            i.clone(),
            123,
            300,
            Side::Ask,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o_passive);
        assert_eq!(r.0, OrderState::Inserted);

        let r = target.add_order(o_aggressive);
        assert_eq!(r.0, OrderState::PartiallyTraded);

        assert_eq!(0, target.generate_bids().len());

        let asks = target.generate_asks();
        assert_eq!(1, asks.len());
        assert_eq!(1001, asks[0].participant);
        assert_eq!(123, asks[0].price);
        assert_eq!(200, asks[0].quantity);
        assert_eq!(Side::Ask, asks[0].side);
        assert_eq!(OrderType::Day, asks[0].order_type);

        assert_eq!(500, asks[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            asks[0].instrument.borrow().get_type()
        );
    }

    #[test]
    fn post_if_same_side() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o_passive = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_aggressive = Order::new(
            1001,
            i.clone(),
            123,
            300,
            Side::Bid,
            OrderType::Day,
            100,
            2001,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o_passive);
        assert_eq!(r.0, OrderState::Inserted);

        let r = target.add_order(o_aggressive);
        assert_eq!(r.0, OrderState::Inserted);

        assert_eq!(0, target.generate_asks().len());

        let bids = target.generate_bids();
        assert_eq!(2, bids.len());
        //first order
        assert_eq!(1000, bids[0].participant);
        assert_eq!(123, bids[0].price);
        assert_eq!(100, bids[0].quantity);
        assert_eq!(Side::Bid, bids[0].side);
        assert_eq!(OrderType::Day, bids[0].order_type);

        assert_eq!(500, bids[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            bids[0].instrument.borrow().get_type()
        );

        // second order
        assert_eq!(1001, bids[1].participant);
        assert_eq!(123, bids[1].price);
        assert_eq!(300, bids[1].quantity);
        assert_eq!(Side::Bid, bids[1].side);
        assert_eq!(OrderType::Day, bids[1].order_type);

        assert_eq!(500, bids[1].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            bids[1].instrument.borrow().get_type()
        );
    }

    #[test]
    fn cross_against_multiple_passive() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o_passive1 = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_passive2 = Order::new(
            1001,
            i.clone(),
            123,
            200,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_passive3 = Order::new(
            1002,
            i.clone(),
            123,
            300,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let o_aggressive = Order::new(
            1003,
            i.clone(),
            123,
            400,
            Side::Ask,
            OrderType::Day,
            101,
            2002,
        );

        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        let r = target.add_order(o_passive1);
        assert_eq!(r.0, OrderState::Inserted);
        let r = target.add_order(o_passive2);
        assert_eq!(r.0, OrderState::Inserted);
        let r = target.add_order(o_passive3);
        assert_eq!(r.0, OrderState::Inserted);

        let r = target.add_order(o_aggressive);
        assert_eq!(r.0, OrderState::Traded);

        assert_eq!(0, target.generate_asks().len());

        // the remainder order
        let bids = target.generate_bids();
        assert_eq!(1, bids.len());
        assert_eq!(1002, bids[0].participant);
        assert_eq!(123, bids[0].price);
        assert_eq!(200, bids[0].quantity);
        assert_eq!(Side::Bid, bids[0].side);
        assert_eq!(OrderType::Day, bids[0].order_type);

        assert_eq!(500, bids[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            bids[0].instrument.borrow().get_type()
        );

        // check the feed
        assert_eq!(3, disseminator.borrow().trades.borrow().len());
    }

    #[test]
    fn cross_partially_against_multiple_passive_and_post() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o_passive1 = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_passive2 = Order::new(
            1001,
            i.clone(),
            123,
            200,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o_passive3 = Order::new(
            1002,
            i.clone(),
            123,
            300,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let o_aggressive = Order::new(
            1003,
            i.clone(),
            123,
            900,
            Side::Ask,
            OrderType::Day,
            200,
            1000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();

        let r = target.add_order(o_passive1);
        assert_eq!(r.0, OrderState::Inserted);
        let r = target.add_order(o_passive2);
        assert_eq!(r.0, OrderState::Inserted);
        let r = target.add_order(o_passive3);
        assert_eq!(r.0, OrderState::Inserted);

        let r = target.add_order(o_aggressive);
        assert_eq!(r.0, OrderState::PartiallyTraded);

        assert_eq!(0, target.generate_bids().len());

        let asks = target.generate_asks();
        assert_eq!(1, asks.len());
        assert_eq!(1003, asks[0].participant);
        assert_eq!(123, asks[0].price);
        assert_eq!(300, asks[0].quantity);
        assert_eq!(Side::Ask, asks[0].side);
        assert_eq!(OrderType::Day, asks[0].order_type);

        assert_eq!(500, asks[0].instrument.borrow().get_id());
        assert_eq!(
            InstrumentType::Share,
            asks[0].instrument.borrow().get_type()
        );
    }

    #[test]
    fn close_deletes_orders() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        let _ = (0..100).map(|_| {
            assert_eq!(OrderState::Inserted, target.add_order(o.clone()).0);
        });
        target.close();

        assert_eq!(0, target.generate_bids().len());
        assert_eq!(0, target.generate_asks().len());
    }

    #[test]
    fn close_closes_instrument() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        let _ = (0..100).map(|_| {
            assert_eq!(OrderState::Inserted, target.add_order(o.clone()).0);
        });
        target.close();

        assert_eq!(InstrumentState::Closed, i.borrow().get_state());
    }

    #[test]
    fn reject_if_out_of_price_bands() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o2 = Order::new(
            1000,
            i.clone(),
            1001,
            100,
            Side::Ask,
            OrderType::Day,
            100,
            2000,
        );
        let o = Order::new(
            1000,
            i.clone(),
            123,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        assert_eq!(OrderState::Inserted, target.add_order(o1).0);
        assert_eq!(OrderState::Inserted, target.add_order(o2).0);
        assert_eq!(OrderState::Rejected, target.add_order(o).0);
    }

    #[test]
    fn market_order_not_inserted() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        let o = Order::new(
            1000,
            i.clone(),
            0,
            1000,
            Side::Bid,
            OrderType::Market,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        assert_eq!(OrderState::Cancelled, target.add_order(o).0);
    }

    #[test]
    fn market_order_not_inserted_after_trade() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o2 = Order::new(
            1000,
            i.clone(),
            1001,
            100,
            Side::Ask,
            OrderType::Day,
            100,
            2000,
        );
        let o = Order::new(
            1000,
            i.clone(),
            0,
            1000,
            Side::Bid,
            OrderType::Market,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        assert_eq!(OrderState::Inserted, target.add_order(o1).0);
        assert_eq!(OrderState::Inserted, target.add_order(o2).0);
        assert_eq!(OrderState::Traded, target.add_order(o).0);
    }

    #[test]
    fn market_order_trades() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o2 = Order::new(
            1000,
            i.clone(),
            1001,
            100,
            Side::Ask,
            OrderType::Day,
            100,
            2000,
        );
        let o = Order::new(
            1000,
            i.clone(),
            0,
            100,
            Side::Bid,
            OrderType::Market,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        assert_eq!(OrderState::Inserted, target.add_order(o1).0);
        assert_eq!(OrderState::Inserted, target.add_order(o2).0);
        assert_eq!(OrderState::Traded, target.add_order(o).0);
    }

    #[test]
    fn order_id_increments() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o2 = Order::new(
            1000,
            i.clone(),
            1001,
            100,
            Side::Ask,
            OrderType::Day,
            100,
            2000,
        );
        let o3 = Order::new(
            1000,
            i.clone(),
            0,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let mut target = Market::new(i.clone(), Rc::new(RefCell::new(MockDisseminator::new())));
        target.set_state_trading();
        assert_eq!(OrderState::Inserted, target.add_order(o1).0);
        assert_eq!(1, target.get_order_id());
        assert_eq!(OrderState::Inserted, target.add_order(o2).0);
        assert_eq!(2, target.get_order_id());
        assert_eq!(OrderState::Rejected, target.add_order(o3).0);
        assert_eq!(3, target.get_order_id());
    }

    #[test]
    fn publish_new_order() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        assert_eq!(1, disseminator.borrow().new_orders.borrow().len());
        assert_eq!(0, disseminator.borrow().cancels.borrow().len());

        o1.set_id(target.get_order_id()); // fix the order id
        assert_eq!(disseminator.borrow().new_orders.borrow()[0], o1);
    }

    #[test]
    fn cancel_invalid_order() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        assert_eq!(OrderState::Rejected, target.cancel_order(&o1));
    }

    #[test]
    fn cancel_valid_order() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        assert_eq!(OrderState::Cancelled, target.cancel_order(&o1));
    }

    #[test]
    fn cancel_invalid_order_side() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.side = Side::Ask;
        assert_eq!(OrderState::Rejected, target.cancel_order(&o1));
    }

    #[test]
    fn cancel_publishes() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        assert_eq!(OrderState::Cancelled, target.cancel_order(&o1));
        assert_eq!(1, disseminator.borrow().new_orders.borrow().len());
        assert_eq!(1, disseminator.borrow().cancels.borrow().len());
    }

    #[test]
    fn modify_invalid_order() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        assert_eq!(OrderState::Rejected, target.modify_order(o1).0);
    }

    #[test]
    fn modify_valid_order() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.price = 990;
        assert_eq!(OrderState::Inserted, target.modify_order(o1).0);

        let bids = target.generate_bids();
        assert_eq!(1, bids.len());
        assert_eq!(990, bids[0].price);
    }

    #[test]
    fn modify_to_zero_quantity_reject() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.quantity = 0;
        assert_eq!(OrderState::Rejected, target.modify_order(o1).0);

        let bids = target.generate_bids();

        // let's make sure that the original order is still there
        // and its price is the original one
        assert_eq!(1, bids.len());
        assert_eq!(1000, bids[0].price);
    }

    #[test]
    fn modify_cant_change_sides() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.side = Side::Ask;
        assert_eq!(OrderState::Rejected, target.modify_order(o1).0);

        // let's make sure that the original order is still there
        assert_eq!(1, target.generate_bids().len());
        assert_eq!(0, target.generate_asks().len());
    }

    #[test]
    fn modify_cant_change_order_type() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.order_type = OrderType::FillOrKill;
        assert_eq!(OrderState::Rejected, target.modify_order(o1).0);

        // let's make sure that the original order is still there
        assert_eq!(1, target.generate_bids().len());
    }

    #[test]
    fn modify_quantity_doesnt_change_position() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let mut o2 = Order::new(
            1000,
            i.clone(),
            1000,
            200,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        assert_eq!(OrderState::Inserted, target.add_order(o2.clone()).0);
        o2.set_id(target.get_order_id());

        // modify the first order quantity should keep it in the first position
        o1.quantity = 50;
        assert_eq!(OrderState::Modified, target.modify_order(o1).0);

        let bids = target.generate_bids();
        assert_eq!(2, bids.len());
        assert_eq!(50, bids[0].quantity);
        assert_eq!(200, bids[1].quantity);
    }

    #[test]
    fn modify_valid_order_invalid_side() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.side = Side::Ask;
        assert_eq!(OrderState::Rejected, target.modify_order(o1).0);
    }

    #[test]
    fn modify_valid_order_publish_delete_new() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.price += 1;
        assert_eq!(OrderState::Inserted, target.modify_order(o1).0);

        assert_eq!(1, disseminator.borrow().cancels.borrow().len());
        assert_eq!(2, disseminator.borrow().new_orders.borrow().len());
    }

    #[test]
    fn modify_quantity_publish_modify() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        o1.quantity += 100;
        assert_eq!(OrderState::Modified, target.modify_order(o1).0);

        assert_eq!(1, disseminator.borrow().modifies.borrow().len());
    }

    #[test]
    fn publish_instrument_and_market() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let mut o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let mut o2 = Order::new(
            1001,
            i.clone(),
            990,
            200,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );

        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        o1.set_id(target.get_order_id());
        assert_eq!(OrderState::Inserted, target.add_order(o2.clone()).0);
        o2.set_id(target.get_order_id());

        let r = target.publish_snapshot();
        assert!(r.is_ok());
        assert_eq!(3, r.unwrap());
        assert_eq!(1, disseminator.borrow().instrument_info.borrow().len());
        assert_eq!(2, disseminator.borrow().market_orders.borrow().len());
    }

    #[test]
    fn cancel_all_orders_for_session() {
        let i = Rc::new(RefCell::new(Instrument::new_fast(
            500,
            InstrumentType::Share,
        )));
        i.borrow_mut().set_percentage_bands(10);

        let o1 = Order::new(
            1000,
            i.clone(),
            1000,
            100,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o2 = Order::new(
            1000,
            i.clone(),
            990,
            200,
            Side::Bid,
            OrderType::Day,
            100,
            2000,
        );
        let o3 = Order::new(
            1001,
            i.clone(),
            1010,
            300,
            Side::Ask,
            OrderType::Day,
            100,
            2001,
        );
        let o4 = Order::new(
            1001,
            i.clone(),
            1020,
            400,
            Side::Ask,
            OrderType::Day,
            101,
            2002,
        );

        let disseminator = Rc::new(RefCell::new(MockDisseminator::new()));
        let mut target = Market::new(i.clone(), disseminator.clone());
        target.set_state_trading();

        assert_eq!(OrderState::Inserted, target.add_order(o1.clone()).0);
        assert_eq!(OrderState::Inserted, target.add_order(o2.clone()).0);
        assert_eq!(OrderState::Inserted, target.add_order(o3.clone()).0);
        assert_eq!(OrderState::Inserted, target.add_order(o4.clone()).0);

        // Verify initial state
        assert_eq!(2, target.generate_bids().len());
        assert_eq!(2, target.generate_asks().len());

        // Cancel all orders for participant 1000, gateway 100, session 2000
        let r = target.cancel_all_orders_for_session(1000, 100, 2000);
        assert_eq!(2, r.len());

        // Verify state after cancellation
        let bids = target.generate_bids();
        let asks = target.generate_asks();

        assert_eq!(0, bids.len());
        assert_eq!(2, asks.len());

        // Verify remaining orders
        assert_eq!(1001, asks[0].participant);
        assert_eq!(1010, asks[0].price);
        assert_eq!(300, asks[0].quantity);
        assert_eq!(1001, asks[1].participant);
        assert_eq!(1020, asks[1].price);
        assert_eq!(400, asks[1].quantity);

        // Verify operation counters
        assert_eq!(6, target.bids_ops);
        assert_eq!(2, target.asks_ops);
    }
}
