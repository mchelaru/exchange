/// For now the clearing engine is not doing too much except
/// downloading some instruments from the database and distributing them
/// to the matching engine
use clearing_connection::genericclearingprotocol::ProtocolSide;
use configparser::ini::Ini;
use disseminator::mockdisseminator::MockDisseminator;
use polling::{Event, Events, PollMode, Poller};
use socket2::Socket;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use std::{collections::BTreeMap, error::Error, io::Read, os::fd::AsRawFd};

use clearing_connection::genericclearingprotocol::GenericClearingProtocol;
use clearing_connection::{
    clearclearingconnection::ClearClearingConnection, clearingconnection::ClearingConnection,
    clearprotocol::ClearProtocol,
};
use instruments::{genericinstrumentlist::GenericInstrumentList, instrumentlist::InstrumentList};
use market::Market;
use utils::config;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Loading configuration file");
    let mut config = Ini::new();
    let config_map = config
        .load("clearing.ini")
        .expect("Unable to load the configuration file");
    let clearing_addr = config::get_config_string(&config_map, "clearing", "address");
    let clearing_port = config::get_config_string(&config_map, "clearing", "port")
        .parse::<u16>()
        .expect("Clearing port must be an u16");
    let max_packet_size = config::get_config_string(&config_map, "clearing", "max_packet_size")
        .parse::<u16>()
        .expect("max_packet_size must be an u16") as usize;

    println!("Starting the clearing server");
    let poller = Poller::new()?;
    let mut poll_events = Events::new();

    let mut instrument_list = InstrumentList::new();

    // Load the instruments
    eprintln!("Connecting to DB");
    let db_type = config::get_config_string(&config_map, "database", "type");
    let db_addr = config::get_config_string(&config_map, "database", "address");
    let db_port = config::get_config_string(&config_map, "database", "port")
        .parse::<u16>()
        .expect("Database port must be an u16");
    let db_user = config::get_config_string(&config_map, "database", "username");
    let db_pass = config::get_config_string(&config_map, "database", "password");
    let db_name = config::get_config_string(&config_map, "database", "name");
    let instrument_refresh =
        config::get_config_string(&config_map, "database", "instrument_refresh")
            .parse::<u64>()
            .expect("instrument_refresh must be a positive integer");

    let mut db_client = dbhook::factory::build(&db_type);
    db_client.connect(&db_addr, db_port, &db_user, &db_pass, &db_name)?;
    eprintln!("Downloading instruments");
    let instruments = db_client.get_instruments();
    eprintln!("Downloaded {} instruments", instruments.len());
    instruments.into_iter().for_each(|i| {
        instrument_list.add_instrument(i);
    });
    let last_update = Instant::now();

    let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));
    let mut protocol = Box::new(ClearProtocol::<InstrumentList>::new(
        instrument_list,
        markets,
        Rc::new(RefCell::new(MockDisseminator::new())), // we don't need a real one here
    ));
    protocol.set_protocol_side(ProtocolSide::Server);
    let mut connection =
        ClearClearingConnection::new(&clearing_addr, clearing_port, Some(protocol));
    connection.listen()?;
    connection.register_with_poller(&poller)?;
    let clearing_socket_fd = connection.get_socket_key();
    let mut clients: BTreeMap<usize, Socket> = BTreeMap::new();

    let mut remaining = HashMap::<usize, Vec<u8>>::new();
    println!("Listening for incoming connections");
    loop {
        poll_events.clear();
        poller.wait(&mut poll_events, Some(Duration::from_secs(1)))?;
        for ev in poll_events.iter() {
            match ev.key {
                k if k == clearing_socket_fd => {
                    // accept
                    let (socket, sockaddr) = connection.accept()?;
                    println!(
                        "Accepted incoming connection from {}",
                        sockaddr.as_socket_ipv4().unwrap().ip() // TODO: IPv6
                    );
                    socket.set_nonblocking(true)?;
                    socket.set_nodelay(true)?;
                    let socket_key = (&socket).as_raw_fd() as usize;
                    unsafe {
                        poller.add_with_mode(
                            &socket,
                            Event::readable(socket_key).with_interrupt(),
                            PollMode::Level,
                        )?;
                    }
                    clients.insert(socket_key, socket);
                    remaining.insert(socket_key, vec![]);
                }
                k if k != clearing_socket_fd => {
                    let mut socket = clients.get(&k).expect("Invalid socket in poll");
                    macro_rules! clean_socket {
                        () => {
                            poller.delete(socket)?;
                            clients.remove(&k);
                            remaining.remove(&k);
                            println!("Disconnected one client");
                            continue;
                        };
                    }
                    if ev.is_interrupt() {
                        clean_socket!();
                    }
                    let mut buffer = Vec::with_capacity(max_packet_size);
                    buffer.resize_with(max_packet_size, Default::default);
                    match socket.read(&mut buffer) {
                        Ok(r) => remaining
                            .get_mut(&k)
                            .unwrap()
                            .append(&mut buffer[0..r].to_vec()),
                        Err(_) => {
                            clean_socket!();
                        }
                    }
                    match connection.process(&remaining.get(&k).unwrap(), Some(socket)) {
                        Ok(bytes) => {
                            let r = remaining.len();
                            if bytes < r {
                                remaining.insert(k, buffer[bytes..r].to_vec());
                            }
                        }
                        Err(e) => {
                            println!("Error {e} reading on socket {:#?}", socket);
                            clean_socket!();
                        }
                    }
                }
                _ => {
                    panic!("Got poll event on invalid socket")
                }
            }
        }

        // every X seconds redownload the instruments and serve them on all the connections
        // TODO: in the end, we need to find a better way of doing this operation:
        // 1. don't send updates for instruments that haven't been updated in the database
        // 2. send updates when an instrument is changed instead of every X seconds
        if Instant::now().duration_since(last_update) > Duration::from_secs(instrument_refresh) {
            let instruments = db_client.get_instruments();
            let response = instruments
                .iter()
                .map(|x| {
                    connection.add_instrument(x.clone());
                    connection
                        .get_protocol()
                        .as_ref()
                        .unwrap()
                        .prepare_instrument_update_response(&x)
                })
                .reduce(|mut acc, mut i| {
                    (&mut acc).append(&mut i);
                    acc
                });
            if response.is_some() {
                clients.iter().for_each(|(_, socket)| {
                    socket.send(response.as_ref().unwrap().as_slice()).unwrap();
                });
            }
        }
    }
}
