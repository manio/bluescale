use beep::beep;
use bluer::{AdapterEvent, Address};
use chrono::{NaiveDate, Utc};
use clap::Parser;
use futures::{pin_mut, StreamExt};
use ini::Ini;
use simplelog::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::{thread, time};
use tokio::time::sleep;

mod bluetooth;
mod body;
mod database;
use crate::bluetooth::query_device;
use crate::body::Person;
use crate::database::Database;

pub const SECS_PER_YEAR: u32 = 31557600;

/// Simple program to read `Xiaomi Mi Body Composition Scale 2` via bluetooth
/// and store measurement in the configured PostgreSQL database
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Enable debug info
    #[clap(short, long)]
    debug: bool,

    /// Config file path
    #[clap(short, long, parse(from_os_str), default_value = "/etc/bluescale.conf")]
    config: std::path::PathBuf,
}

fn logging_init(debug: bool) {
    let conf = ConfigBuilder::new()
        .set_time_format("%F, %H:%M:%S%.3f".to_string())
        .set_write_log_enable_colors(true)
        .build();

    let mut loggers = vec![];

    let console_logger: Box<dyn SharedLogger> = TermLogger::new(
        if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        conf.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );
    loggers.push(console_logger);

    CombinedLogger::init(loggers).expect("Cannot initialize logging subsystem");
}

fn config_read_postgres(conf: Ini) -> Result<Database, Box<dyn std::error::Error>> {
    match conf.section(Some("postgres".to_owned())) {
        Some(section) => Ok(Database {
            name: "🦏 postgres".to_string(),
            host: section.get("host").ok_or("missing `host`")?.to_string(),
            dbname: section.get("dbname").ok_or("missing `dbname`")?.to_string(),
            username: section
                .get("username")
                .ok_or("missing `username`")?
                .to_string(),
            password: section
                .get("password")
                .ok_or("missing `password`")?
                .to_string(),
        }),
        None => Err("missing [postgres] config section")?,
    }
}

fn config_read_profile(conf: Ini) -> Result<Person, Box<dyn std::error::Error>> {
    match conf.section(Some("profile".to_owned())) {
        Some(section) => {
            //computing age:
            let date_str = section.get("birthday").ok_or("missing `birthday`")?;
            let birthday = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
            if let Err(e) = birthday {
                return Err(format!("error parsing `birthday`: {}", e).into());
            }
            let d = Utc::now().date_naive() - birthday?;
            let age = d.num_seconds() as f32 / SECS_PER_YEAR as f32;
            debug!("Age calculated from {:?} birthday: {:?}", date_str, age);

            Ok(Person {
                sex: section.get("sex").ok_or("missing `sex`")?.parse()?,
                age,
                height: section.get("height").ok_or("missing `height`")?.parse()?,
            })
        }
        None => Err("missing [profile] config section")?,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    logging_init(args.debug);
    info!("<b><blue>bluescale</> started");
    info!("Using config file: <b><blue>{:?}</>", args.config);
    let conf = match Ini::load_from_file(args.config) {
        Ok(c) => c,
        Err(e) => {
            error!("Cannot open config file: {}", e);
            return Ok(());
        }
    };

    let mut db = match config_read_postgres(conf.clone()) {
        Ok(db) => db,
        Err(e) => {
            return Err(format!("Config error [postgres]: {}", e).into());
        }
    };
    let p = match config_read_profile(conf.clone()) {
        Ok(p) => {
            info!("👤 Using profile: {}", p);
            p
        }
        Err(e) => {
            return Err(format!("Config error [profile]: {}", e).into());
        }
    };

    let mut filter_addr: HashSet<bluer::Address> = HashSet::new();
    let mac = conf
        .section(Some("miscale".to_owned()))
        .unwrap_or(&HashMap::new())
        .get("mac")
        .unwrap_or(&String::new())
        .to_string()
        .parse::<Address>()
        .ok();
    if let Some(addr) = mac {
        info!("Filtering devices to MAC: {:?}", addr);
        filter_addr.insert(addr);
    } else {
        info!("Scale MAC address not provided or parse error, all devices will be probed");
    }

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    info!(
        "🛰️  Discovering devices using Bluetooth adapter <b>{}</b> ...",
        adapter.name()
    );
    adapter.set_powered(true).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        if !filter_addr.is_empty() && !filter_addr.contains(&addr) {
                            continue;
                        }

                        info!("📳 Device added: {}", addr);
                        _ = std::thread::spawn(|| {
                            let _ = beep(580);
                            thread::sleep(time::Duration::from_millis(100));
                            let _ = beep(680);
                            thread::sleep(time::Duration::from_millis(100));
                            let _ = beep(780);
                            thread::sleep(time::Duration::from_millis(100));
                            let _ = beep(0);
                        });

                        info!("👣 Sleeping and waiting for data: {}", addr);
                        sleep(std::time::Duration::from_secs(10)).await;

                        for i in 1..=10 {
                            if let Err(e) = query_device(&adapter, addr, &mut db, p.clone()).await {
                                warn!("Device query error (try: {}/10): {}", i, e);
                                thread::sleep(time::Duration::from_millis(1500));
                                continue;
                            } else {
                                _ = std::thread::spawn(|| {
                                    let _ = beep(440);
                                    thread::sleep(time::Duration::from_millis(300));
                                    let _ = beep(880);
                                    thread::sleep(time::Duration::from_millis(200));
                                    let _ = beep(0);
                                });
                                break;
                            }
                        };
                    }
                    AdapterEvent::DeviceRemoved(addr) => {
                        if !filter_addr.is_empty() && !filter_addr.contains(&addr) {
                            continue;
                        }
                        info!("💤 Device removed: {}", addr);
                    }
                    _ => (),
                }
            }
            else => break
        }
    }

    Ok(())
}
