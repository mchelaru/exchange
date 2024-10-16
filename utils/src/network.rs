use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::net::Ipv4Addr;
use std::os::fd::AsFd;
use std::rc::Rc;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

pub fn join_multicast_group(group_addr: &SockAddr) -> io::Result<Socket> {
    let socket = Socket::new(group_addr.domain(), Type::DGRAM, Some(Protocol::UDP))?;

    match group_addr.domain() {
        Domain::IPV4 => {
            socket.join_multicast_v4(
                group_addr
                    .as_socket_ipv4()
                    .expect("Group address cannot be a socket")
                    .ip(),
                &Ipv4Addr::UNSPECIFIED,
            )?;
        }
        Domain::IPV6 => {
            socket.join_multicast_v6(group_addr.as_socket_ipv6().unwrap().ip(), 0)?;
            socket.set_only_v6(true)?;
        }
        _ => return Err(std::io::ErrorKind::AddrNotAvailable.into()),
    };

    socket.bind(group_addr)?;
    Ok(socket)
}

#[derive(Debug, Clone)]
pub struct MockSocket {
    // we read from this buffer
    pub read_buffer: RefCell<Vec<u8>>,
    // and we write in this buffer, unless output is set to something
    pub write_buffer: RefCell<Vec<u8>>,
    // if this is set, a write call will append to the target read_buffer
    pub output: Option<Rc<RefCell<MockSocket>>>,
    // if this is set then read returns UnexpectedEof and write returns UnexpectedEof
    pub closed: bool,
}

impl MockSocket {
    pub fn new() -> Self {
        Self {
            read_buffer: RefCell::new(vec![]),
            write_buffer: RefCell::new(vec![]),
            output: None,
            closed: false,
        }
    }

    ///
    /// Connects the outputs of this socket to the input of a different one,
    /// that is, the writes on this sockets will land in the read_buffer of the
    /// @output MockSocket
    ///
    pub fn connect_output(&mut self, output: Rc<RefCell<MockSocket>>) {
        self.output = Some(output);
    }

    ///
    /// Closes the MockSocket. No more read or writes are allowed and
    /// subsequent calls to them will return Err(UnexpectedEof)
    ///
    pub fn close(&mut self) {
        self.closed = true;
    }
}

impl Read for MockSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.closed {
            Err(std::io::ErrorKind::UnexpectedEof.into())
        } else {
            let r = std::cmp::min(self.read_buffer.borrow().len(), buf.len());
            buf[..r].clone_from_slice(&self.read_buffer.borrow().as_slice()[..r]);
            self.read_buffer.borrow_mut().drain(..r);

            Ok(r)
        }
    }
}

impl Write for MockSocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.closed {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        if self.output.is_none() {
            self.write_buffer.borrow_mut().append(&mut buf.to_vec());
        } else {
            self.output
                .as_ref()
                .unwrap()
                .borrow_mut()
                .read_buffer
                .borrow_mut()
                .append(&mut buf.to_vec());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsFd for MockSocket {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        todo!()
    }
}
