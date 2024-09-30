use configparser::ini::Ini;
use disseminator::mbooepdisseminator::MBOOepDisseminator;
use instruments::genericinstrumentlist::GenericInstrumentList;
use oep::decoder::Decoder;
use oep::execution_report::EXECUTIONREPORT_SIZE;
use oep::header::{OepHeader, OEP_VERSION};
use oep::oep_message::MsgType;
use polling::{Event, Events, PollMode, Poller};

#[cfg(feature = "usdt")]
use usdt::{dtrace_provider, register_probes};

use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use clearing_connection::clearclearingconnection::ClearClearingConnection;
use clearing_connection::clearingconnection::ClearingConnection;
use clearing_connection::clearprotocol::ClearProtocol;
use clearing_connection::genericclearingprotocol::GenericClearingProtocol;
use instruments::instrumentlist::InstrumentList;
use market::Market;
use utils::config;
use utils::network;

mod processor;

fn main() -> Result<(), Box<dyn Error>> {
    // load USDTs
    #[cfg(feature = "usdt")]
    dtrace_provider!("matching_engine_probes.d");
    #[cfg(feature = "usdt")]
    register_probes().unwrap();
    #[cfg(feature = "usdt")]
    macro_rules! timeit {
        ($probe: ident, $func: expr) => {{
            let start = Instant::now();
            let result = $func;
            let duration = start.elapsed().as_nanos() as u64;
            matching::$probe!(|| (duration));
            result
        }};
    }
    #[cfg(not(feature = "usdt"))]
    macro_rules! timeit {
        ($_probe: ident, $func: expr) => {{
            $func
        }};
    }

    println!("Loading configuration file");
    let mut config = Ini::new();
    let config_map = config
        .load("matching_engine.ini")
        .expect("Unable to load the configuration file");
    let disseminator_addr = config::get_config_string(&config_map, "engine", "disseminator_group");
    let disseminator_port = config::get_config_string(&config_map, "engine", "disseminator_port")
        .parse::<u16>()
        .expect("Disseminator port must be an u16");
    let max_packet_size = config::get_config_string(&config_map, "engine", "max_packet_size")
        .parse::<u16>()
        .expect("max_packet_size must be an u16") as usize;
    let order_addr = config::get_config_string(&config_map, "engine", "order_group");
    let order_port = config::get_config_string(&config_map, "engine", "order_port")
        .parse::<u16>()
        .expect("Order port must be an u16");

    let internal_publisher_addr =
        config::get_config_string(&config_map, "engine", "internal_publisher_group");
    let internal_publisher_port =
        config::get_config_string(&config_map, "engine", "internal_publisher_port")
            .parse::<u16>()
            .expect("Internal publisher port must be an u16 integer");

    let clearing_addr = config::get_config_string(&config_map, "clearing", "address");
    let clearing_port = config::get_config_string(&config_map, "clearing", "port")
        .parse::<u16>()
        .expect("Clearing port must be an u16");

    println!("Starting the engine");
    let poller = Poller::new()?;
    let mut poll_events = Events::new();
    let markets = Rc::new(RefCell::new(HashMap::<u64, Market>::new()));

    println!("Connecting to clearing");
    // we will use the "Clear" protocol
    let protocol_h = Box::new(ClearProtocol::new(
        InstrumentList::new(),
        markets.clone(),
        Rc::new(RefCell::new(MBOOepDisseminator::new(
            &disseminator_addr,
            disseminator_port,
        ))),
    )) as Box<dyn GenericClearingProtocol>;
    let mut clearing_connection =
        ClearClearingConnection::new(&clearing_addr, clearing_port, Some(protocol_h));
    clearing_connection.connect()?;
    clearing_connection.register_with_poller(&poller)?;
    let clearing_socket_fd = clearing_connection.get_socket_key();

    println!("Requesting the instrument list");
    clearing_connection.request_instruments()?;

    // at this point we have an instrument list, so theoretically we can accept orders
    println!("Preparing order socket");
    let mut order_socket = network::join_multicast_group(&SockAddr::from(SocketAddr::V4(
        SocketAddrV4::new(Ipv4Addr::from_str(&order_addr)?, order_port),
    )))?;
    let order_socket_fd = order_socket.as_raw_fd() as usize;
    unsafe {
        poller.add_with_mode(
            &order_socket,
            Event::readable(order_socket_fd),
            PollMode::Level,
        )?;
    }

    let mut internal_publisher_socket =
        Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    internal_publisher_socket.connect(&SockAddr::from(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::from_str(&internal_publisher_addr).unwrap(),
        internal_publisher_port,
    ))))?;
    internal_publisher_socket
        .set_multicast_loop_v4(true)
        .expect("set_multicast_loop_v4");

    // the main loop
    println!("Ready to trade");
    let mut read_buffer = Vec::with_capacity(max_packet_size);
    read_buffer.resize_with(max_packet_size, Default::default);

    let mut clearing_buffer = vec![];

    const SEND_SNAPSHOTS_EVERY_MS: Duration = Duration::from_millis(20000);
    let mut last_snapshot_sent = Instant::now() - Duration::from_millis(18000);

    let execution_report_header = OepHeader {
        oep_version: OEP_VERSION,
        msg_type: MsgType::ExecutionReport.into(),
        msg_len: EXECUTIONREPORT_SIZE as u32,
    }
    .encode();

    loop {
        poll_events.clear();
        poller.wait(&mut poll_events, Some(Duration::from_millis(500)))?;
        for ev in poll_events.iter() {
            match ev.key {
                k if k == order_socket_fd => {
                    let r = order_socket.read(&mut read_buffer).unwrap_or_default();
                    if r > 3 {
                        let msg_result =
                            timeit!(decode, processor::decode_message(&read_buffer[0..r]));
                        match msg_result {
                            Ok((msg, book_id)) => match markets.borrow_mut().get_mut(&book_id) {
                                Some(market) => {
                                    let ereport =
                                        timeit!(process, processor::process_message(market, msg));
                                    timeit!(
                                        publish,
                                        internal_publisher_socket.write(
                                            [
                                                execution_report_header.as_slice(),
                                                ereport.encode().as_slice(),
                                            ]
                                            .concat()
                                            .as_slice(),
                                        )?
                                    );
                                }
                                None => {}
                            },
                            Err(_) => {}
                        }
                    };
                }
                k if k == clearing_socket_fd => {
                    let r = clearing_connection
                        .read(&mut read_buffer)
                        .unwrap_or_default();
                    clearing_buffer.append(&mut read_buffer[0..r].to_vec());
                    match timeit!(
                        clearing_process,
                        clearing_connection.process(&clearing_buffer, None)
                    ) {
                        Ok(bytes) => {
                            assert!(bytes <= clearing_buffer.len());
                            clearing_buffer.drain(0..bytes);
                        }
                        Err(e) => {
                            eprintln!("Clearing message decoding error {}", e);
                        }
                    }
                }
                _ => panic!("Got event on unknown socket"),
            }
        }
        // send snapshots around if needed
        if last_snapshot_sent.elapsed() > SEND_SNAPSHOTS_EVERY_MS {
            eprintln!("Sending snapshots for {} markets", markets.borrow().len());
            timeit!(
                send_snapshots,
                markets.borrow().iter().for_each(|(_id, m)| {
                    if m.publish_snapshot().is_err() {
                        eprintln!("Error publishing instrument snapshot");
                    }
                })
            );
            last_snapshot_sent = Instant::now();
        }
    }
}
