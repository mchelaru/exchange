use std::{error::Error, io};

use instruments::instrument::Instrument;
use socket2::Socket;

use crate::clearingconnection::ClearingConnection;

use super::genericclearingprotocol::GenericClearingProtocol;

pub struct MockClearingConnection {}

impl ClearingConnection for MockClearingConnection {
    fn new(_addr: &str, _port: u16, _proto: Option<Box<dyn GenericClearingProtocol>>) -> Self {
        Self {}
    }

    fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn request_instruments(&self) -> Result<usize, Box<dyn Error>> {
        todo!()
        // d.insert(instrument::Instrument::new(100, InstrumentType::Share));
        // Ok(1)
    }

    fn register_with_poller(&mut self, _poller: &polling::Poller) -> io::Result<()> {
        Ok(())
    }

    fn get_socket_key(&self) -> usize {
        usize::MAX
    }

    fn process(
        &mut self,
        _buffer: &[u8],
        _response_socket: Option<&Socket>,
    ) -> Result<usize, super::genericclearingprotocol::ProcessError> {
        todo!()
    }

    fn get_protocol(&self) -> &Option<Box<dyn GenericClearingProtocol>> {
        todo!()
    }

    fn accept(&self) -> io::Result<(socket2::Socket, socket2::SockAddr)> {
        todo!()
    }

    fn add_instrument(&mut self, _i: Instrument) {
        todo!()
    }
}

impl std::io::Read for MockClearingConnection {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl std::io::Write for MockClearingConnection {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}
