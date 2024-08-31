use std::error::Error;
use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct DecodeError;
impl Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Decoding Error")
    }
}
impl Error for DecodeError {}

pub trait Decoder<const S: usize>
where
    Self: Sized + Clone + Copy,
{
    fn encode(self) -> [u8; S];
    fn decode(buffer: [u8; S]) -> Result<Self, Box<dyn Error>>;
}
