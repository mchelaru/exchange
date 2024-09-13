use instruments::instrument::Instrument;
use oep::trade::Trade;
use order::Order;

pub trait Disseminator: std::fmt::Debug {
    fn send_cancel_order(&self, order: &Order) -> Result<usize, std::io::Error>;
    fn send_new_order(&self, order: &Order) -> Result<usize, std::io::Error>;
    fn send_modify_order(&self, trade: &Order) -> Result<usize, std::io::Error>;
    fn send_trade(&self, trade: &Trade) -> Result<usize, std::io::Error>;

    // instruments and snapshots
    fn send_instrument_info(&self, instruments: &Instrument) -> Result<usize, std::io::Error>;
    // sends market update, order by order
    fn send_market_order(&self, order: &Order) -> Result<usize, std::io::Error>;
}
