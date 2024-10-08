use std::{
    cell::RefCell,
    ffi::CString,
    io::{Read, Write},
    os::fd::AsFd,
    rc::Rc,
};

use anyhow::{bail, Result};
use dbhook::genericdb::GenericDB;
use oep::{
    cancel::Cancel,
    decoder::Decoder,
    header::{OepHeader, OEP_VERSION},
    login::Login,
    modify::Modify,
    neworder::NewOrder,
    oep_message::{MsgType, OepMessage},
};
use polling::AsSource;

pub struct ConnectedSession<TSocket>
where
    TSocket: Read + Write + AsFd + AsSource,
{
    pub(crate) socket: Rc<RefCell<TSocket>>,
    pub(crate) session_id: u32,
    pub(crate) participant: u64,
    #[allow(unused)] // the mocksocket may not want to use the recv_buffer
    pub(crate) recv_buffer: Rc<RefCell<Vec<u8>>>,
    pub response_buffer: Vec<u8>,
    pub(crate) is_corked: bool,
    cork_buf: Vec<u8>,
}

impl<TSocket: Read + Write + AsFd + AsSource> ConnectedSession<TSocket> {
    pub fn new(socket: Rc<RefCell<TSocket>>) -> Self {
        Self {
            socket: socket.clone(),
            session_id: 0,
            participant: 0,
            recv_buffer: Rc::new(RefCell::new(Vec::with_capacity(500))),
            response_buffer: Vec::with_capacity(500),
            is_corked: false,
            cork_buf: vec![],
        }
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        if !self.is_corked {
            self.socket.borrow_mut().write(buf)
        } else {
            self.cork_buf.append(&mut buf.to_vec());
            Ok(buf.len())
        }
    }

    pub fn cork(&mut self) {
        self.is_corked = true;
    }

    pub fn uncork(&mut self) -> Result<usize, std::io::Error> {
        let mut sent = 0;
        if self.cork_buf.len() > 0 {
            sent = self.socket.borrow_mut().write(self.cork_buf.as_slice())?;
            self.cork_buf.clear();
        }
        self.is_corked = false;
        Ok(sent)
    }
}

/// for a login message: returns an updated participant ID in case login was successful
/// or 0 if login failed. For the rest of the messages, returns the participant ID.
/// If message is not accepted => Err
/// IMPORTANT: the message that needs to be relayed to the matching engine will
/// end up in the session.response_buffer. It is up to the caller to actually
/// send this message to the matching engine and clear the response_buffer afterwards.
///
/// @session is mutable for a number of reasons:
///  * it is adjusting the session_id and the participant_id in case of a successful login
///  * it is sending back a login response in case the login request was successful
///  * it is populating the response_buffer in case we need to send something to the matching engine
///
/// Arguments:
///
/// @db - a connected datebase session
/// @session - a client session that received the message
/// @message - the OEP message that we received
///
/// Returns:
///     the participant id
#[must_use]
pub fn receive_and_prepare_relay_message<TSocket: Read + Write + AsFd + AsSource>(
    db: &mut Box<dyn GenericDB>,
    session: &mut ConnectedSession<TSocket>,
    message: &Box<dyn OepMessage>,
) -> Result<u64> {
    macro_rules! relay_message {
        ($message: expr, $msgtype: ty, $msg_type_encoding: expr) => {
            let msg: &$msgtype = $message
                .as_any()
                .downcast_ref::<$msgtype>()
                .expect("Bad pointer conversion");
            session
                .response_buffer
                .extend_from_slice(&[$msg_type_encoding as u8, 0, 0, 0]);
            session.response_buffer.extend_from_slice(&msg.encode());
        };
    }

    macro_rules! check_session {
        () => {
            if message.get_participant() != session.participant || session.participant == 0 {
                bail!("Invalid participant");
            }
        };
    }

    match message.message_type() {
        MsgType::Login => {
            if session.participant == 0 {
                // we need to check the login
                let msg = message
                    .as_any()
                    .downcast_ref::<Login>()
                    .expect("Bad pointer conversion");
                let session_id = msg.session_id;
                session.session_id = session_id;
                // TODO: check if already logged in
                let mut v: Vec<u8> = msg.user.to_vec().into_iter().filter(|x| *x != 0).collect();
                v.push(0);
                session.participant = db.check_login(
                    &CString::from_vec_with_nul(v)
                        .expect("receive_message cstring::new")
                        .into_string()
                        .expect("receive_message into_string"),
                    &msg.password,
                    session_id,
                )?;
                println!("Successful login for participant {}", session.participant);

                // send the response back as the original login message with a
                // standard header
                session.cork();
                session.send(
                    OepHeader::new(
                        OEP_VERSION,
                        MsgType::Login.into(),
                        oep::login::LOGIN_SIZE as u32,
                    )
                    .encode()
                    .as_slice(),
                )?;
                session.send(&msg.encode())?;
                session.uncork()?;
            } else {
                // already logged in
                bail!("Already logged in");
            }
        }
        MsgType::Cancel => {
            check_session!();
            relay_message!(message, Cancel, 2);
        }
        MsgType::Modify => {
            check_session!();
            relay_message!(message, Modify, 1);
        }
        MsgType::NewOrder => {
            check_session!();
            relay_message!(message, NewOrder, 0);
        }
        MsgType::ExecutionReport => {
            eprintln!(
                "Ignoring received execution report from participant {} on session {}",
                session.participant, session.session_id
            );
            bail!("execution report message received");
        }
        MsgType::Trade => {
            eprintln!(
                "Ignoring received trade message from participant {} on session {}",
                session.participant, session.session_id
            );
            bail!("trade message received");
        }
        _ => {
            eprintln!(
                "Ignoring received unknown message type from participant {} on session {}",
                session.participant, session.session_id
            );
            bail!("unknown message received");
        }
    }

    return Ok(session.participant);
}
