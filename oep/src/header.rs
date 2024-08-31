use std::error::Error;

use crate::{decoder::Decoder, oep_message::MsgType};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct OepHeader {
    pub oep_version: u16,
    pub msg_type: u16,
    pub msg_len: u32,
}

pub const OEP_VERSION: u16 = 1;
pub const OEP_HEADER_SIZE: usize = std::mem::size_of::<OepHeader>();

impl OepHeader {
    pub fn new(oep_version: u16, msg_type: u16, msg_len: u32) -> Self {
        Self {
            oep_version: oep_version,
            msg_type: msg_type,
            msg_len: msg_len,
        }
    }

    pub fn message_type(&self) -> MsgType {
        match self.msg_type {
            0 => MsgType::NewOrder,
            1 => MsgType::Modify,
            2 => MsgType::Cancel,
            3 => MsgType::ExecutionReport,
            4 => MsgType::Login,
            _ => MsgType::Unknown,
        }
    }
}

impl Decoder<OEP_HEADER_SIZE> for OepHeader {
    fn encode(self) -> [u8; OEP_HEADER_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; OEP_HEADER_SIZE]>(self) }
    }

    fn decode(buffer: [u8; OEP_HEADER_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; OEP_HEADER_SIZE], Self>(buffer).try_into()?) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{decoder::Decoder, header::MsgType};

    use super::OepHeader;

    #[test]
    fn decode() {
        let header_bytes = [1, 0, 2, 0, 20, 0, 0, 0];
        let boxed_target = OepHeader::decode(header_bytes);
        assert!(boxed_target.is_ok());
        let target = boxed_target.unwrap();
        let oep_version = target.oep_version;
        let msg_type = target.msg_type;
        let msg_len = target.msg_len;

        assert_eq!(oep_version, 1);
        assert_eq!(msg_type, 2);
        assert_eq!(msg_len, 20);
    }

    #[test]
    fn deduce_login() {
        let header_bytes = [1, 0, 4, 0, 20, 0, 0, 0];
        let target = OepHeader::decode(header_bytes);

        assert!(target.is_ok());
        assert_eq!(target.unwrap().message_type(), MsgType::Login);
    }
}
