use anyhow::Result;
use configparser::ini::Ini;
use connection_factory::{ConnectionFactory, EventType};
use oep::{
    decoder::Decoder,
    execution_report::{ExecutionReport, EXECUTIONREPORT_SIZE},
    header::{OepHeader, OEP_HEADER_SIZE},
    oep_decode,
    oep_message::MsgType,
};
use polling::Events;
use socket2::Protocol;
use std::{io::Read, mem::MaybeUninit, os::fd::AsRawFd};

use utils::config::get_config_string;
pub mod messages;
use messages::receive_and_prepare_relay_message;
mod connection_factory;

const MAX_READ_ARRAY_SIZE: usize = 15000;

/// reinterpret a maybeuninit slice to an u8 slice
/// as seen at https://github.com/rust-lang/socket2/blob/master/tests/socket.rs
unsafe fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
    &*(buf as *const [MaybeUninit<u8>] as *const [u8])
}

fn main() -> Result<()> {
    //read configuration file
    println!(
        "Loading configuration file. Current dir is {}",
        std::env::current_dir()?.display()
    );
    let mut config = Ini::new();
    let config_map = config
        .load("gateway.ini")
        .expect("Unable to load the configuration file");

    // gateway section
    let gateway_id = get_config_string(&config_map, "gateway", "id")
        .parse::<u32>()
        .expect("Gateway ID must be an integer") as u8;
    let gateway_addr = get_config_string(&config_map, "gateway", "address");
    let gateway_port = get_config_string(&config_map, "gateway", "port")
        .parse::<u16>()
        .expect("Gateway port must be an u16");
    let gateway_publisher_addr = get_config_string(&config_map, "gateway", "publisher_addr");
    let gateway_publisher_port = get_config_string(&config_map, "gateway", "publisher_port")
        .parse::<u16>()
        .expect("Publisher port must be an u16");
    let max_packet_size = get_config_string(&config_map, "gateway", "max_packet_size")
        .parse::<u16>()
        .expect("max_packet_size port must be an u16") as usize;
    assert!(max_packet_size <= MAX_READ_ARRAY_SIZE);

    // internal publisher section
    let internal_publisher_addr =
        get_config_string(&config_map, "gateway", "internal_publisher_group");
    let internal_publisher_port =
        get_config_string(&config_map, "gateway", "internal_publisher_port")
            .parse::<u16>()
            .expect("Internal publisher port must be an u16 integer");

    // database section
    let dbtype = get_config_string(&config_map, "database", "type");
    let dbport = get_config_string(&config_map, "database", "port")
        .parse::<u16>()
        .expect("Invalid port in the database section");
    let dbaddr = get_config_string(&config_map, "database", "address");
    let dbuser = get_config_string(&config_map, "database", "username");
    let dbpass = get_config_string(&config_map, "database", "password");
    let dbname = get_config_string(&config_map, "database", "database");

    // connect to DB
    println!("Connecting to DB");
    let mut db = dbhook::factory::build(&dbtype);
    db.connect(&dbaddr, dbport, &dbuser, &dbpass, &dbname)?;

    // create sockets and poller
    println!("Initializing sockets");

    let mut connection_factory = ConnectionFactory::new();
    let listener = connection_factory.add_tcp_listener(&gateway_addr, gateway_port)?;
    let listener_raw_fd = listener.socket.borrow().as_raw_fd() as usize;

    let sender_raw_fd = connection_factory
        .add_socket(
            Protocol::UDP,
            &gateway_publisher_addr,
            gateway_publisher_port,
            false,
            None,
        )?
        .socket
        .borrow()
        .as_raw_fd() as usize;

    let mut poll_events = Events::new();

    println!("Preparing internal publisher socket");
    // we use this socket in order to receive messages back from the matching engine
    let internal_publisher_raw_fd = connection_factory
        .add_socket(
            Protocol::UDP,
            &internal_publisher_addr,
            internal_publisher_port,
            true,
            Some(EventType::Read),
        )?
        .socket
        .borrow()
        .as_raw_fd() as usize;

    let mut read_buffer = [MaybeUninit::<u8>::uninit(); MAX_READ_ARRAY_SIZE];

    // this blob of code is virtually untestable
    // TODO: split it out, use the ConnectionFactory instead
    println!("Polling");
    loop {
        connection_factory.poll(&mut poll_events, None)?;
        for ev in poll_events.iter() {
            match ev.key {
                k if k == listener_raw_fd => {
                    connection_factory.accept(listener_raw_fd, Some(EventType::Read))?;
                    println!("New client accepted");
                }
                k if k == internal_publisher_raw_fd => {
                    let mut buf = [0; 10000];
                    let internal_publisher_socket =
                        connection_factory.get_mut_session_by_client_fd(k).unwrap();
                    let r = internal_publisher_socket
                        .socket
                        .borrow_mut()
                        .read(&mut buf)
                        .unwrap();
                    if r < OEP_HEADER_SIZE {
                        continue;
                    }
                    // theoretically we should receive only execution reports here, but let's check
                    let oep_header =
                        OepHeader::decode(buf[0..OEP_HEADER_SIZE].try_into().unwrap()).unwrap();
                    if oep_header.message_type() != MsgType::ExecutionReport
                        || r != OEP_HEADER_SIZE + EXECUTIONREPORT_SIZE
                    {
                        eprintln!("Non-execution report received from the matching engine!");
                        continue;
                    }
                    let ereport =
                        ExecutionReport::decode(buf[OEP_HEADER_SIZE..r].try_into().unwrap())
                            .unwrap();
                    // quickly check if we're the target for this message
                    if ereport.gateway_id != gateway_id {
                        continue;
                    }
                    // send it further down the wire to the interested client
                    let session_id = ereport.session_id;
                    match connection_factory.get_mut_session_by_session_id(session_id) {
                        Some(connection) => {
                            let _ = connection.send(&buf[0..r]);
                        }
                        None => {
                            eprintln!("Received message from matching engine for invalid session_id {session_id}!");
                        } // drop
                    }
                }
                k => {
                    let cf = connection_factory.get_mut_session_by_client_fd(k);
                    match cf {
                        Some(ref p) => {
                            // we stop referencing p here, since it holds connection_factory mut ref
                            let client_socket = p.socket.clone();
                            let participant = p.participant;
                            let session = p.session_id;
                            let prev_buffer = p.recv_buffer.clone();
                            drop(cf); // we drop the connection_factory here, since we want to borrow it again down below

                            let read_result = client_socket.borrow_mut().recv(&mut read_buffer);
                            if let Ok(r) = read_result {
                                let vbuf = unsafe { assume_init(&read_buffer[..r]) };
                                prev_buffer.borrow_mut().extend_from_slice(vbuf);
                                let m = oep_decode(&prev_buffer.borrow());

                                match m {
                                    Ok(msg) => {
                                        prev_buffer
                                            .borrow_mut()
                                            .drain(0..msg.message_len() + OEP_HEADER_SIZE);

                                        // check if the message was addressed to the right gateway
                                        if msg.get_gateway_id() != gateway_id {
                                            println!(
                                                "Message was sent for a different gateway({})",
                                                msg.get_gateway_id()
                                            );
                                            connection_factory.delete_socket(k);
                                            continue;
                                        }
                                        // check the message session_id if this was set
                                        if session != 0 && msg.get_session_id() != session {
                                            println!(
                                                "Message was sent for a different session({})",
                                                msg.get_session_id()
                                            );
                                            connection_factory.delete_socket(k);
                                            continue;
                                        }

                                        if (participant == 0 || session == 0)
                                            && msg.message_type() != MsgType::Login
                                        {
                                            eprintln!("Expected login, received something else. Closing client socket.");
                                            connection_factory.delete_socket(k);
                                        } else {
                                            let p = connection_factory
                                                .get_mut_session_by_client_fd(k)
                                                .unwrap();
                                            match receive_and_prepare_relay_message(
                                                &mut db, p, &msg,
                                            ) {
                                                Ok(new_participant) => {
                                                    if participant == 0 && new_participant != 0 {
                                                        // login successful, need to update the session id mapping

                                                        connection_factory.update_session_id(
                                                            msg.get_session_id(),
                                                            k,
                                                        );
                                                        continue;
                                                    } else if participant != 0 {
                                                        // regular message, check if we have to relay something to the matching engine
                                                        if p.response_buffer.len() > 0 {
                                                            // we get rid of referencing p here, since it holds connection_factory - needed below
                                                            let local_buffer_copy = std::mem::take(
                                                                &mut p.response_buffer,
                                                            );

                                                            let sender = connection_factory
                                                                .get_mut_session_by_client_fd(
                                                                    sender_raw_fd,
                                                                )
                                                                .unwrap();
                                                            sender.send(&local_buffer_copy)?;
                                                        }
                                                    } else if participant == 0 {
                                                        // login failed
                                                        eprintln!("Login failed");
                                                        connection_factory.delete_socket(k);
                                                    }
                                                }
                                                Err(err) => {
                                                    println!("Invalid message from participant {participant}: {err}. Closing connection.");
                                                    connection_factory.delete_socket(k);
                                                }
                                            }
                                        }
                                    }
                                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                                        if r == 0 {
                                            println!("EOF, closing connection");
                                            connection_factory.delete_socket(k);
                                        }
                                        // otherwise no-op, we just cache what we have and try again when we have more data
                                    }
                                    Err(e) => {
                                        println!("Client sent an invalid command, closing its socket. Error: {e}");
                                        connection_factory.delete_socket(k);
                                    }
                                }
                            } else {
                                println!("Session {session} disconnected");
                                connection_factory.delete_socket(k);
                            }
                        }
                        None => {
                            panic!("Invalid socket received in poll");
                        }
                    }
                }
            }
        }
    }
}
