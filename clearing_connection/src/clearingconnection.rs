use instruments::instrument::Instrument;
use socket2::{SockAddr, Socket};
use std::error::Error;
use std::io;

use super::genericclearingprotocol::{GenericClearingProtocol, ProcessError};

pub trait ClearingConnection: std::io::Read + std::io::Write {
    fn new(addr: &str, port: u16, proto: Option<Box<dyn GenericClearingProtocol>>) -> Self;
    fn connect(&mut self) -> Result<(), Box<dyn Error>>;
    fn listen(&mut self) -> Result<(), Box<dyn Error>>;
    fn accept(&self) -> io::Result<(Socket, SockAddr)>;
    fn register_with_poller(&mut self, poller: &polling::Poller) -> std::io::Result<()>;
    fn get_socket_key(&self) -> usize;
    fn process(
        &mut self,
        buffer: &[u8],
        response_socket: Option<&Socket>,
    ) -> Result<usize, ProcessError>;

    // returns number of instruments added
    fn request_instruments(&self) -> Result<usize, Box<dyn Error>>;
    fn add_instrument(&mut self, i: Instrument);
    fn get_protocol(&self) -> &Option<Box<dyn GenericClearingProtocol>>;
}
