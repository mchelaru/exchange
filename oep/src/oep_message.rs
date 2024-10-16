use crate::{
    cancel::CANCEL_SIZE, execution_report::EXECUTIONREPORT_SIZE, login::LOGIN_SIZE,
    modify::MODIFY_SIZE, neworder::NEWORDER_SIZE, sessioninfo::SESSIONINFO_SIZE, trade::TRADE_SIZE,
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MsgType {
    NewOrder,
    Modify,
    Cancel,
    ExecutionReport,
    Login,
    Trade,
    SessionNotification, // sent by GW to ME, in order to inform the latter about a session exception
    Unknown,
}

impl Into<u16> for MsgType {
    fn into(self) -> u16 {
        match self {
            MsgType::NewOrder => 0,
            MsgType::Modify => 1,
            MsgType::Cancel => 2,
            MsgType::ExecutionReport => 3,
            MsgType::Login => 4,
            // MsgType::Trade intentionally left out
            // Msg::SessionInfo intentionall left out
            _ => panic!("Unknown message type"),
        }
    }
}

impl From<u16> for MsgType {
    fn from(value: u16) -> Self {
        match value {
            0 => MsgType::NewOrder,
            1 => MsgType::Modify,
            2 => MsgType::Cancel,
            3 => MsgType::ExecutionReport,
            4 => MsgType::Login,
            5 => MsgType::Trade,
            6 => MsgType::SessionNotification,
            _ => MsgType::Unknown,
        }
    }
}

pub trait OepMessage {
    fn message_type(&self) -> MsgType;
    fn message_len(&self) -> usize {
        match self.message_type() {
            MsgType::Cancel => CANCEL_SIZE,
            MsgType::ExecutionReport => EXECUTIONREPORT_SIZE,
            MsgType::Login => LOGIN_SIZE,
            MsgType::Modify => MODIFY_SIZE,
            MsgType::NewOrder => NEWORDER_SIZE,
            MsgType::Trade => TRADE_SIZE,
            MsgType::SessionNotification => SESSIONINFO_SIZE,
            MsgType::Unknown => 1024,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any;
    fn get_gateway_id(&self) -> u8;
    fn get_session_id(&self) -> u32;
    fn get_participant(&self) -> u64;
}
