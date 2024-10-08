use crate::messages::ConnectedSession;
use anyhow::{anyhow, bail, Result};
use polling::{Event, Events, PollMode, Poller};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, Read},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    os::fd::AsRawFd,
    rc::Rc,
    str::FromStr,
    time::Duration,
};

pub struct ConnectionFactory {
    client_fd_to_session: HashMap<usize, ConnectedSession<Socket>>,
    session_id_to_client_fd: HashMap<u32, usize>,
    poller: Poller,
}

pub enum EventType {
    Read,
    #[allow(dead_code)]
    Write,
    #[allow(dead_code)]
    ReadWrite,
}

impl ConnectionFactory {
    pub fn new() -> Self {
        Self {
            client_fd_to_session: HashMap::new(),
            session_id_to_client_fd: HashMap::new(),
            poller: Poller::new().unwrap(),
        }
    }

    /// get the connected session from the file descriptor
    pub fn get_session_by_client_fd(&self, fd: usize) -> Option<&ConnectedSession<Socket>> {
        self.client_fd_to_session.get(&fd)
    }

    /// get the mutable connected session from the file descriptor
    pub fn get_mut_session_by_client_fd(
        &mut self,
        fd: usize,
    ) -> Option<&mut ConnectedSession<Socket>> {
        self.client_fd_to_session.get_mut(&fd)
    }

    /// get the connected session from the session id
    #[allow(unused)]
    pub fn get_session_by_session_id(&self, session_id: u32) -> Option<&ConnectedSession<Socket>> {
        match self.session_id_to_client_fd.get(&session_id) {
            Some(client_fd) => self.client_fd_to_session.get(&client_fd),
            None => None,
        }
    }

    pub fn get_mut_session_by_session_id(
        &mut self,
        session_id: u32,
    ) -> Option<&mut ConnectedSession<Socket>> {
        match self.session_id_to_client_fd.get_mut(&session_id) {
            Some(client_fd) => self.client_fd_to_session.get_mut(&client_fd),
            None => None,
        }
    }

    /// Inserts a socket into the client session map (self.client_fd_to_session).
    /// Returns a reference to the connected session
    fn insert_fd_to_session(&mut self, socket: Socket) -> Result<&ConnectedSession<Socket>> {
        let key = socket.as_raw_fd() as usize;
        self.client_fd_to_session
            .insert(key, ConnectedSession::new(Rc::new(RefCell::new(socket))));
        self.client_fd_to_session
            .get(&key)
            .ok_or(anyhow!("client_fd_to_session insert error"))
    }

    /// call this after the login process is completed, in order to update
    /// the session_id -> session resolution, so this session can receive
    /// back execution reports coming from the matching engine
    pub fn update_session_id(&mut self, session_id: u32, client_fd: usize) {
        self.session_id_to_client_fd.insert(session_id, client_fd);
    }

    pub fn add_socket(
        &mut self,
        protocol: Protocol,
        address: &str,
        port: u16,
        multicast: bool,
        event: Option<EventType>,
    ) -> Result<&ConnectedSession<Socket>> {
        if multicast {
            assert_eq!(protocol, Protocol::UDP);
            let socket = utils::network::join_multicast_group(&SockAddr::from(SocketAddr::V4(
                SocketAddrV4::new(Ipv4Addr::from_str(&address)?, port),
            )))?;
            self.add_to_poller(&socket, event)?;
            self.insert_fd_to_session(socket)
        } else {
            let socket = match protocol {
                Protocol::UDP => {
                    Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap()
                }
                Protocol::TCP => {
                    Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::UDP)).unwrap()
                }
                _ => bail!("protocol type not supported yet"),
            };

            socket.connect(&SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from_str(&address).unwrap(),
                port,
            ))))?;
            self.add_to_poller(&socket, event)?;
            self.insert_fd_to_session(socket)
        }
    }

    fn add_to_poller(&mut self, socket: &Socket, event: Option<EventType>) -> io::Result<()> {
        match event {
            Some(event) => unsafe {
                let key = socket.as_raw_fd() as usize;
                self.poller.add_with_mode(
                    socket,
                    match event {
                        EventType::Read => Event::readable(key),
                        EventType::Write => Event::writable(key),
                        EventType::ReadWrite => Event::all(key),
                    },
                    PollMode::Level,
                )
            },
            None => Ok(()),
        }
    }

    pub fn delete_socket(&mut self, socket_key: usize) {
        let target_session: Option<ConnectedSession<Socket>> =
            self.client_fd_to_session.remove(&socket_key);
        match target_session {
            Some(t) => {
                let _ = self.poller.delete(t.socket.borrow_mut().by_ref());
                self.session_id_to_client_fd.remove(&t.session_id);
            }
            None => {}
        }
    }

    pub fn add_tcp_listener(&mut self, addr: &str, port: u16) -> Result<&ConnectedSession<Socket>> {
        let listener = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
        listener.set_linger(None)?;
        listener.set_reuse_address(true)?;
        listener.set_reuse_port(true)?;
        listener.bind(&SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from_str(&addr).unwrap(),
            port,
        ))))?;
        listener.listen(10)?;

        // automatically register with the poller on read events
        unsafe {
            self.poller.add_with_mode(
                &listener,
                Event::readable(listener.as_raw_fd() as usize),
                PollMode::Level,
            )?;
        }

        self.insert_fd_to_session(listener)
    }

    /// Clears the @poll_events and starts another poll
    pub fn poll(&mut self, poll_events: &mut Events, timeout: Option<Duration>) -> Result<usize> {
        poll_events.clear();
        Ok(self.poller.wait(poll_events, timeout)?)
    }

    pub fn accept(
        &mut self,
        listener_fd: usize,
        event: Option<EventType>,
    ) -> Result<&ConnectedSession<Socket>> {
        match self.get_session_by_client_fd(listener_fd) {
            Some(listener) => {
                let socket = listener.socket.borrow().accept()?.0;
                socket.set_nonblocking(true)?;
                socket.set_nodelay(true)?;
                self.add_to_poller(&socket, event)?;

                self.insert_fd_to_session(socket)
            }
            None => bail!("No socket found"),
        }
    }
}
