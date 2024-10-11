use std::{
    io::Read,
    net::{Ipv4Addr, SocketAddrV4},
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Completion, FuzzySelect, Input};
use instruments::instrument::Instrument;
use oep::{cancel::Cancel, connection::MessageTypes, modify::Modify, neworder::NewOrder};

use configparser::ini::Ini;
use order::OrderType;
use socket2::SockAddr;
use utils::config;
use utils::network;

struct InstrumentCompletion {
    options: Vec<String>,
}

impl InstrumentCompletion {
    fn new(instrument_list: &Vec<Instrument>) -> Self {
        InstrumentCompletion {
            options: instrument_list
                .iter()
                .map(|x| String::from(x.get_name()))
                .collect(),
        }
    }
}

impl Completion for InstrumentCompletion {
    /// Simple completion implementation based on substring
    fn get(&self, input: &str) -> Option<String> {
        let matches = self
            .options
            .iter()
            .filter(|option| option.starts_with(input))
            .collect::<Vec<_>>();

        if matches.len() == 1 {
            Some(matches[0].to_string())
        } else {
            None
        }
    }
}

fn main() -> Result<()> {
    //read configuration file
    println!("Loading configuration file");
    let mut config = Ini::new();
    let config_map = config
        .load("client.ini")
        .expect("Unable to load the configuration file");

    let instruments: Arc<Mutex<Vec<Instrument>>> = Arc::new(Mutex::new(vec![]));

    let feed_group = config::get_config_string(&config_map, "feed", "group");
    let feed_port = config::get_config_string(&config_map, "feed", "port")
        .parse::<u16>()
        .expect("Feed port not an u16");

    let instrument_list = instruments.clone();
    thread::spawn(move || {
        let mut listener = network::join_multicast_group(&SockAddr::from(
            std::net::SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from_str(&feed_group).expect("Invalid feed group address"),
                feed_port,
            )),
        ))
        .expect("Couldn't create the listener");
        let mut buffer: [u8; 2000] = [0; 2000];
        loop {
            let r = listener
                .read(&mut buffer)
                .expect("read error from the feed socket");
            if r > 8 && buffer[8] == 1 {
                // we got ourselves an instrument update
                let instrument_slice = &buffer[9..r];
                let instrument = Instrument::decode(
                    instrument_slice
                        .try_into()
                        .expect("Instrument decoding error"),
                );
                // let's try figuring out if we already have the instrument or if it's a new one
                let mut ilist = instrument_list.lock().expect("ilist lock");
                let mut found_at = ilist.len();
                for (i, e) in ilist.iter().enumerate() {
                    if e.get_id() == instrument.get_id() {
                        found_at = i;
                        break;
                    }
                }
                eprintln!("Received instrument {:#?}", instrument);
                if found_at == ilist.len() {
                    ilist.push(instrument);
                } else {
                    ilist[found_at] = instrument;
                }
            }
        }
    });

    // Gateway section
    eprintln!("Connecting to GW");
    let gw_addr = config::get_config_string(&config_map, "gateway", "address");
    let gw_port = config::get_config_string(&config_map, "gateway", "port")
        .parse::<u16>()
        .expect("Gateway port must be an u16");
    let gw_user = config::get_config_string(&config_map, "gateway", "username");
    let gw_pass = config::get_config_string(&config_map, "gateway", "password");
    let gw_participant = config::get_config_string(&config_map, "gateway", "participant")
        .parse::<u64>()
        .expect("Gateway participant must be an u64");
    let gw_session_id = config::get_config_string(&config_map, "gateway", "session_id")
        .parse::<u32>()
        .expect("Gateway session_id must be an u32");
    let gw_gateway_id = config::get_config_string(&config_map, "gateway", "gateway_id")
        .parse::<u8>()
        .expect("Gateway gateway_id must be an u8");

    let mut connection = oep::connection::Connection::default();
    connection.connect(&gw_addr, gw_port)?;
    connection.login(
        gw_participant,
        gw_session_id,
        gw_gateway_id,
        &gw_user,
        &gw_pass,
    )?;
    connection.wait_for_login(Some(5000))?;

    macro_rules! get_instrument_id {
        () => {{
            // FIXME: we copy out the instrument list here, since we don't want to deadlock below
            let instrument_list_copy = instruments.lock().expect("ilcopy lock").clone();
            let instrument = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Instrument")
                .completion_with(&InstrumentCompletion::new(&instrument_list_copy))
                .validate_with(|i: &String| -> Result<(), &str> {
                    if instrument_list_copy
                        .iter()
                        .find(|x| x.get_name() == i)
                        .is_some()
                    {
                        Ok(())
                    } else {
                        Err("No such instrument")
                    }
                })
                .interact_text()
                .unwrap();
            let instrument_id = instruments
                .lock()
                .unwrap()
                .iter()
                .find(|x| x.get_name() == instrument)
                .unwrap()
                .get_id();
            (instrument, instrument_id)
        }};
    }

    loop {
        let choices = ["new_order", "modify", "cancel", "quit"];

        let selection = FuzzySelect::new()
            .with_prompt("Message type")
            .items(&choices)
            .interact()
            .unwrap();

        match choices[selection] {
            "new_order" => {
                let order_type = ["day", "ioc"];
                let selection = FuzzySelect::new()
                    .with_prompt("Order type")
                    .items(&order_type)
                    .interact()
                    .unwrap();
                let order_type = order_type[selection];
                let (instrument, instrument_id) = get_instrument_id!();
                let side = ["bid", "ask"];
                let selection = FuzzySelect::new()
                    .with_prompt("Side")
                    .items(&side)
                    .interact()
                    .unwrap();
                let side = side[selection];

                let quantity = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Quantity")
                    .interact_text()
                    .unwrap();
                let price = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Price")
                    .interact_text()
                    .unwrap();

                println!(
                    "Your order: {} {} {} {}@{}",
                    order_type, side, instrument, quantity, price
                );
                let order = MessageTypes::NewOrder(NewOrder {
                    client_order_id: 0,
                    participant: gw_participant,
                    book_id: instrument_id,
                    quantity: quantity,
                    price: price,
                    order_type: if order_type == "day" {
                        OrderType::Day.into()
                    } else {
                        OrderType::FillAndKill.into()
                    },
                    side: if side == "buy" { 0 } else { 1 },
                    gateway_id: gw_gateway_id,
                    session_id: gw_session_id,
                });
                connection.send_message(order)?;
            }
            "modify" => {
                let order_id = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Order ID:")
                    .interact_text()
                    .unwrap();
                let (_instrument, instrument_id) = get_instrument_id!();
                let quantity = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Quantity")
                    .interact_text()
                    .unwrap();
                let price = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Price")
                    .interact_text()
                    .unwrap();
                let side = ["bid", "ask"];
                let selection_side = FuzzySelect::new()
                    .with_prompt("Side")
                    .items(&side)
                    .interact()
                    .unwrap();
                let order = MessageTypes::Modify(Modify {
                    participant: gw_participant,
                    order_id: order_id,
                    book_id: instrument_id,
                    quantity: quantity,
                    price: price,
                    gateway_id: gw_gateway_id,
                    session_id: gw_session_id,
                    side: selection_side as u8,
                });
                connection.send_message(order)?;
            }
            "cancel" => {
                let order_id = Input::<u64>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Order ID:")
                    .interact_text()
                    .unwrap();
                let (_instrument, instrument_id) = get_instrument_id!();
                let side = ["bid", "ask"];
                let selection_side = FuzzySelect::new()
                    .with_prompt("Side")
                    .items(&side)
                    .interact()
                    .unwrap();
                let order = MessageTypes::Cancel(Cancel {
                    participant: gw_participant,
                    order_id: order_id,
                    book_id: instrument_id,
                    gateway_id: gw_gateway_id,
                    session_id: gw_session_id,
                    side: selection_side as u8,
                });
                connection.send_message(order)?;
            }
            "quit" | _ => break,
        }
        // wait 1 second for something from gateway
        match connection.recv_message(Duration::from_secs(1)) {
            Some(m) => println!("{:#?}", m),
            None => {}
        }
    }
    Ok(())
}
