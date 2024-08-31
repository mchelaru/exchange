use anyhow::{bail, Result};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{
    io::Read,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
    time::Duration,
};

use crate::{
    cancel::CANCEL_SIZE,
    decoder::Decoder,
    execution_report::EXECUTIONREPORT_SIZE,
    header::{OepHeader, OEP_HEADER_SIZE, OEP_VERSION},
    login::{Login, LOGIN_SIZE},
    modify::MODIFY_SIZE,
    neworder::NEWORDER_SIZE,
    oep_decode,
    oep_message::MsgType,
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum ConnectionState {
    Disconnected,
    Connected,
    LoginSent,
    Logged,
}

impl ConnectionState {
    fn advance(&mut self) {
        match self {
            ConnectionState::Disconnected => *self = ConnectionState::Connected,
            ConnectionState::Connected => *self = ConnectionState::LoginSent,
            ConnectionState::LoginSent => *self = ConnectionState::Logged,
            ConnectionState::Logged => *self = ConnectionState::Disconnected,
        }
    }
}

#[derive(Debug)]
pub enum MessageTypes {
    Cancel(crate::cancel::Cancel),
    ExecutionReport(crate::execution_report::ExecutionReport),
    Login(crate::login::Login),
    Modify(crate::modify::Modify),
    NewOrder(crate::neworder::NewOrder),
    Trade(crate::trade::Trade),
}

pub struct Connection {
    socket: Option<Socket>,
    state: ConnectionState,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            socket: None,
            state: ConnectionState::Disconnected,
        }
    }
}

impl Connection {
    pub fn connect(&mut self, addr: &str, port: u16) -> Result<()> {
        assert_eq!(ConnectionState::Disconnected, self.state);

        self.socket = Some(Socket::new(
            Domain::IPV4,
            Type::STREAM,
            Some(Protocol::TCP),
        )?);

        self.socket
            .as_mut()
            .unwrap()
            .connect(&SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from_str(&addr).unwrap(),
                port,
            ))))?;

        self.state.advance();

        Ok(())
    }

    pub fn login(
        &mut self,
        participant: u64,
        session_id: u32,
        gateway_id: u8,
        username: &str,
        password: &str,
    ) -> Result<()> {
        assert_eq!(ConnectionState::Connected, self.state);

        let mut msg = Login::new(participant, session_id, gateway_id, username);
        msg.hash_text_to_password(password);
        let header = OepHeader::new(OEP_VERSION, MsgType::Login.into(), LOGIN_SIZE.try_into()?);
        self.socket.as_ref().unwrap().send(&header.encode())?;
        self.socket.as_ref().unwrap().send(&msg.encode())?;
        self.state.advance();

        Ok(())
    }

    pub fn wait_for_login(&mut self, timeout_ms: Option<u64>) -> Result<()> {
        let real_timeout = timeout_ms.unwrap_or(2000);
        self.socket
            .as_ref()
            .unwrap()
            .set_read_timeout(Some(Duration::from_millis(real_timeout)))?;
        let mut buf: [u8; 10000] = [0; 10000];
        let bytes_read = self.socket.as_ref().unwrap().read(&mut buf)?;

        if bytes_read > 0 {
            match oep_decode(&buf[..bytes_read]) {
                Err(e) => bail!("Decode err: {}", e),
                Ok(msg) => match msg.message_type() {
                    crate::MsgType::Login => {
                        self.state.advance();
                        return Ok(());
                    }
                    _ => bail!("Not login"),
                },
            }
        }

        bail!("Too few bytes read");
    }

    pub fn send_message(&self, msg: MessageTypes) -> Result<()> {
        match msg {
            MessageTypes::Login(_) => bail!("Send login using login fn"),
            MessageTypes::NewOrder(order) => {
                let header = OepHeader::new(
                    OEP_VERSION,
                    MsgType::NewOrder.into(),
                    NEWORDER_SIZE.try_into()?,
                );
                self.socket.as_ref().unwrap().send(&header.encode())?;
                self.socket.as_ref().unwrap().send(&order.encode())?;
            }
            MessageTypes::Cancel(order) => {
                let header =
                    OepHeader::new(OEP_VERSION, MsgType::Cancel.into(), CANCEL_SIZE.try_into()?);
                self.socket.as_ref().unwrap().send(&header.encode())?;
                self.socket.as_ref().unwrap().send(&order.encode())?;
            }
            MessageTypes::ExecutionReport(order) => {
                let header = OepHeader::new(
                    OEP_VERSION,
                    MsgType::ExecutionReport.into(),
                    EXECUTIONREPORT_SIZE.try_into()?,
                );
                self.socket.as_ref().unwrap().send(&header.encode())?;
                self.socket.as_ref().unwrap().send(&order.encode())?;
            }
            MessageTypes::Modify(order) => {
                let header =
                    OepHeader::new(OEP_VERSION, MsgType::Modify.into(), MODIFY_SIZE.try_into()?);
                self.socket.as_ref().unwrap().send(&header.encode())?;
                self.socket.as_ref().unwrap().send(&order.encode())?;
            }
            MessageTypes::Trade(_) => bail!("Can't send trades"),
        }

        Ok(())
    }

    /**
     * Receives messages from the gateway - until now it's implmented to
     * receive just execution reports.
     * Blocks for at most twice the duration
     *
     */
    #[must_use]
    pub fn recv_message(&self, duration: Duration) -> Option<MessageTypes> {
        assert_eq!(ConnectionState::Logged, self.state);
        self.socket
            .as_ref()
            .unwrap()
            .set_read_timeout(Some(duration))
            .unwrap();
        let mut header_buf: [u8; OEP_HEADER_SIZE] = [0; OEP_HEADER_SIZE];
        match self.socket.as_ref().unwrap().read_exact(&mut header_buf) {
            Ok(_) => {
                let header = OepHeader::decode(header_buf).unwrap();
                let mut v = Vec::with_capacity(OEP_HEADER_SIZE + header.msg_len as usize);
                match self.socket.as_ref().unwrap().read_exact(&mut v) {
                    Ok(_) => match oep_decode(&v) {
                        Ok(m) => match m.message_type() {
                            crate::oep_message::MsgType::NewOrder => todo!(),
                            crate::oep_message::MsgType::Modify => todo!(),
                            crate::oep_message::MsgType::Cancel => todo!(),
                            // we only care about execution reports for now
                            crate::oep_message::MsgType::ExecutionReport => {
                                return Some(MessageTypes::ExecutionReport(
                                    *m.as_any()
                                        .downcast_ref::<crate::execution_report::ExecutionReport>()
                                        .expect("Bad pointer conversion"),
                                ))
                            }
                            crate::oep_message::MsgType::Login => todo!(),
                            crate::oep_message::MsgType::Trade => todo!(),
                            crate::oep_message::MsgType::Unknown => todo!(),
                        },
                        Err(_) => return None,
                    },
                    Err(_) => return None, // we lose the header here
                }
            }
            Err(_) => return None,
        }
    }
}
