use std::{error::Error, ffi::CString};

use crate::{
    decoder::Decoder,
    oep_message::{MsgType, OepMessage},
};
use sha2::{Digest, Sha512};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Login {
    pub participant: u64,
    pub session_id: u32,
    gateway_id: u8,
    _padding: [u8; 3],
    pub user: [u8; 64],
    pub password: [u8; 64],
}

impl Login {
    pub fn new(participant: u64, session_id: u32, gateway_id: u8, user: &str) -> Self {
        let mut r = Self {
            participant: participant,
            session_id: session_id,
            gateway_id: gateway_id,
            _padding: [0, 0, 0],
            user: [0; 64],
            password: [0; 64],
        };
        assert!(user.len() < 64 - 1);
        r.user[0..user.len() + 1].clone_from_slice(
            CString::new(user)
                .expect("CString failed in Login")
                .as_bytes_with_nul(),
        );
        r
    }

    pub fn hash_text_to_password(&mut self, text: &str) {
        self.password = Login::free_text_hash(text);
    }

    pub fn free_text_hash(text: &str) -> [u8; 64] {
        let mut hasher = Sha512::new();
        hasher.update(text);
        hasher
            .finalize()
            .as_slice()
            .try_into()
            .expect("Invalid slice")
    }
}

pub const LOGIN_SIZE: usize = std::mem::size_of::<Login>();

impl Decoder<LOGIN_SIZE> for Login {
    fn encode(self) -> [u8; LOGIN_SIZE] {
        unsafe { std::mem::transmute::<Self, [u8; LOGIN_SIZE]>(self) }
    }

    fn decode(buffer: [u8; LOGIN_SIZE]) -> Result<Self, Box<dyn Error>> {
        unsafe { Ok(std::mem::transmute::<[u8; LOGIN_SIZE], Self>(buffer).try_into()?) }
    }
}

impl OepMessage for Login {
    fn message_type(&self) -> MsgType {
        MsgType::Login
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_gateway_id(&self) -> u8 {
        self.gateway_id
    }

    fn get_session_id(&self) -> u32 {
        self.session_id
    }

    fn get_participant(&self) -> u64 {
        self.participant
    }
}

#[cfg(test)]
mod tests {
    use super::Login;

    #[test]
    fn hash_password() {
        let mut target = Login::new(0, 0, 0, "user");
        target.hash_text_to_password("abc");
        assert_eq!(
            target.password,
            [
                221, 175, 53, 161, 147, 97, 122, 186, 204, 65, 115, 73, 174, 32, 65, 49, 18, 230,
                250, 78, 137, 169, 126, 162, 10, 158, 238, 230, 75, 85, 211, 154, 33, 146, 153, 42,
                39, 79, 193, 168, 54, 186, 60, 35, 163, 254, 235, 189, 69, 77, 68, 35, 100, 60,
                232, 14, 42, 154, 201, 79, 165, 76, 164, 159
            ]
        );
    }
}
