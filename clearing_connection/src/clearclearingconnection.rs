// The implementation of the "Clear" Connection

use polling::{Event, PollMode};
use socket2::{SockAddr, Socket};
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::os::fd::AsRawFd;
use std::str::FromStr;

use crate::clearingconnection::ClearingConnection;

use super::genericclearingprotocol::{GenericClearingProtocol, ProcessError};

// an implementation of the clear clearing protocol
pub struct ClearClearingConnection {
    connection: Option<Socket>,
    address: String,
    port: u16,
    protocol: Option<Box<dyn GenericClearingProtocol>>,
}

impl ClearingConnection for ClearClearingConnection {
    fn new(addr: &str, port: u16, proto: Option<Box<dyn GenericClearingProtocol>>) -> Self {
        Self {
            connection: None,
            address: String::from(addr),
            port: port,
            protocol: proto,
        }
    }

    fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let clearing_addr = &SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from_str(self.address.as_str()).unwrap(),
            self.port,
        )));
        self.connection = Some(Socket::new(
            clearing_addr.domain(),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?);
        self.connection.as_ref().unwrap().set_reuse_address(true)?;
        self.connection
            .as_ref()
            .expect("Connect: missing socket")
            .connect(clearing_addr)?;

        Ok(())
    }

    fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let clearing_addr = &SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from_str(self.address.as_str()).unwrap(),
            self.port,
        )));
        self.connection = Some(Socket::new(
            clearing_addr.domain(),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?);
        self.connection.as_ref().unwrap().set_reuse_address(true)?;
        self.connection.as_ref().unwrap().set_reuse_port(true)?;
        self.connection
            .as_ref()
            .expect("Listen: missing socket")
            .bind(&clearing_addr)?;
        self.connection
            .as_ref()
            .expect("Listen: missing socket")
            .listen(4)?;

        Ok(())
    }

    fn accept(&self) -> std::io::Result<(Socket, SockAddr)> {
        self.connection.as_ref().unwrap().accept()
    }

    fn register_with_poller(&mut self, poller: &polling::Poller) -> std::io::Result<()> {
        if self.connection.is_some() {
            unsafe {
                poller.add_with_mode(
                    self.connection.as_ref().expect("Poller: missing socket"),
                    Event::readable(self.get_socket_key()),
                    PollMode::Level,
                )
            }
        } else {
            panic!("No clear socket to register")
        }
    }

    fn get_socket_key(&self) -> usize {
        match &self.connection {
            Some(x) => x.as_raw_fd() as usize,
            None => panic!("Trying to get the socket key, but there is no socket"),
        }
    }

    fn request_instruments(&self) -> Result<usize, Box<dyn Error>> {
        let message = self
            .protocol
            .as_ref()
            .unwrap()
            .prepare_all_instrument_request();
        match self.connection.as_ref().unwrap().send(&message) {
            Ok(s) => Ok(s),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn process(
        &mut self,
        buffer: &[u8],
        response_socket: Option<&Socket>,
    ) -> Result<usize, ProcessError> {
        let mut total_bytes = 0;
        loop {
            match self
                .protocol
                .as_mut()
                .unwrap()
                .process(&buffer[total_bytes..])
            {
                Ok((response, bytes)) => {
                    // if any, sends the response back on the provided socket
                    // otherwise use the generic connection
                    if response.len() > 0 {
                        if match response_socket {
                            Some(socket) => socket.send(&response),
                            None => self.connection.as_ref().unwrap().send(&response),
                        }
                        .is_err()
                        {
                            return Err(ProcessError::new("Error sending response"));
                        }
                    }
                    if bytes == 0 {
                        // break on non-progress
                        break Ok(total_bytes);
                    }
                    total_bytes += bytes;
                }
                Err(x) => break Err(x),
            }
        }
    }

    fn get_protocol(&self) -> &Option<Box<dyn GenericClearingProtocol>> {
        &self.protocol
    }

    fn add_instrument(&mut self, i: instruments::instrument::Instrument) {
        self.protocol.as_mut().unwrap().add_instrument(i);
    }
}

impl std::io::Read for ClearClearingConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.connection
            .as_ref()
            .expect("Read: missing socket")
            .read(buf)
    }
}

impl std::io::Write for ClearClearingConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.connection
            .as_ref()
            .expect("Write: missing socket")
            .write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.connection
            .as_ref()
            .expect("Flush: missing socket")
            .flush()
    }
}
