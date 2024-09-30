use std::{cell::RefCell, rc::Rc};

use instruments::instrument::Instrument;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

impl Into<u8> for Side {
    fn into(self) -> u8 {
        match self {
            Self::Bid => 0,
            Self::Ask => 1,
        }
    }
}

impl From<u8> for Side {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Bid,
            1 => Self::Ask,
            _ => Self::Ask,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OrderType {
    // standing day limit order
    Day,
    // fill as much as possible
    Market,
    // fill as much and kill
    FillAndKill,
    // fill all or kill
    FillOrKill,
    // post the order on the passive side or reject
    PostOrKill,
    // good till cancel
    GoodTillCancel,
    // good till date
    GoodTillDate,
    // stop loss
    StopLoss,
    // stop loss limit
    StopLimit,
}

impl Into<u16> for OrderType {
    fn into(self) -> u16 {
        match self {
            OrderType::Day => 0,
            OrderType::Market => 1,
            OrderType::FillAndKill => 2,
            OrderType::FillOrKill => 3,
            OrderType::PostOrKill => 4,
            OrderType::GoodTillCancel => 5,
            OrderType::GoodTillDate => 6,
            OrderType::StopLoss => 7,
            OrderType::StopLimit => 8,
        }
    }
}

impl From<u16> for OrderType {
    fn from(value: u16) -> Self {
        match value {
            0 => OrderType::Day,
            1 => OrderType::Market,
            2 => OrderType::FillAndKill,
            3 => OrderType::FillOrKill,
            4 => OrderType::PostOrKill,
            5 => OrderType::GoodTillCancel,
            6 => OrderType::GoodTillDate,
            7 => OrderType::StopLoss,
            8 => OrderType::StopLimit,
            _ => OrderType::StopLimit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    id: u64,
    pub participant: u64,
    pub instrument: Rc<RefCell<Instrument>>,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
    pub order_type: OrderType,
    pub gateway_id: u8,
    pub session_id: u32,
}

impl Order {
    pub fn new(
        participant: u64,
        instrument: Rc<RefCell<Instrument>>,
        price: u64,
        quantity: u64,
        side: Side,
        order_type: OrderType,
        gateway_id: u8,
        session_id: u32,
    ) -> Self {
        Self {
            id: 0,
            participant: participant,
            instrument: instrument,
            price: price,
            quantity,
            side: side,
            order_type: order_type,
            gateway_id: gateway_id,
            session_id: session_id,
        }
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderState {
    Inserted,
    Modified,
    Cancelled,
    Rejected,
    Traded,
    PartiallyTraded,
}

impl Into<u8> for OrderState {
    fn into(self) -> u8 {
        match self {
            OrderState::Inserted => 0,
            OrderState::Modified => 1,
            OrderState::Cancelled => 2,
            OrderState::Rejected => 3,
            OrderState::Traded => 4,
            OrderState::PartiallyTraded => 5,
        }
    }
}
