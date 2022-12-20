use crate::body::*;
use crate::Database;
use bitops::BitOps;
use bluer::{Adapter, Address};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use simplelog::*;

const SERVICE_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x0000181b00001000800000805f9b34fb);

pub async fn query_device(
    adapter: &Adapter,
    addr: Address,
    db: &mut Database,
    p: Person,
) -> Result<(), Box<dyn std::error::Error>> {
    let device = adapter.device(addr)?;
    debug!("    Address type:       {}", device.address_type().await?);
    debug!("    Name:               {:?}", device.name().await?);
    debug!("    Icon:               {:?}", device.icon().await?);
    debug!("    Class:              {:?}", device.class().await?);
    debug!(
        "    UUIDs:              {:?}",
        device.uuids().await?.unwrap_or_default()
    );
    debug!("    Paired:             {:?}", device.is_paired().await?);
    debug!("    Connected:          {:?}", device.is_connected().await?);
    debug!("    Trusted:            {:?}", device.is_trusted().await?);
    debug!("    Modalias:           {:?}", device.modalias().await?);
    debug!("    RSSI:               {:?}", device.rssi().await?);
    debug!("    TX power:           {:?}", device.tx_power().await?);
    debug!(
        "    Manufacturer data:  {:?}",
        device.manufacturer_data().await?
    );
    let x = device.service_data().await?;
    debug!("    Service data:       {:?}", x);
    //todo: check if vec size=13
    match x {
        Some(x) => parse(x.get(&SERVICE_UUID).unwrap().to_vec(), db, p).await,
        None => Err("No service data".into()),
    }
}

/// Check if a input `DateTime` occurs in range of the specified duration from now.
pub fn in_range(input_dt: DateTime<Utc>, range_dur: Duration) -> bool {
    let utc_now_dt = Utc::now();
    let within_range = input_dt >= utc_now_dt - range_dur && input_dt <= utc_now_dt + range_dur;
    within_range
}

async fn parse(
    data: Vec<u8>,
    db: &mut Database,
    p: Person,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctrl_byte0: u8 = data[0];
    let ctrl_byte1: u8 = data[1];

    let _is_weight_removed: bool = ctrl_byte1.is_bit_set(7);
    let is_date_invalid: bool = ctrl_byte1.is_bit_set(6);
    let is_stabilized: bool = ctrl_byte1.is_bit_set(5);
    let is_lbs_unit: bool = ctrl_byte0.is_bit_set(0);
    let is_catty_unit: bool = ctrl_byte1.is_bit_set(6);
    let is_impedance: bool = ctrl_byte1.is_bit_set(1);

    if is_stabilized && /* is_weight_removed &&*/ !is_date_invalid {
        let year: u16 = ((data[3] as u16) << 8) + data[2] as u16;
        let month: u8 = data[4];
        let day: u8 = data[5];
        let hours: u8 = data[6];
        let min: u8 = data[7];
        let sec: u8 = data[8];

        let weight: f32;
        let mut impedance: f32 = 0.0;

        if is_lbs_unit || is_catty_unit {
            weight = (((data[12] as u16) << 8) + data[11] as u16) as f32 / 100.0;
        } else {
            weight = (((data[12] as u16) << 8) + data[11] as u16) as f32 / 200.0;
        }

        if is_impedance {
            impedance = (((data[10] as u16) << 8) + data[9] as u16) as f32;
            debug!("Impedance value: {}", impedance);
        }

        let date_time: NaiveDateTime = NaiveDate::from_ymd(year as i32, month as u32, day as u32)
            .and_hms(hours as u32, min as u32, sec as u32);

        // is the timestamp plausible? check if it is in the range of 10 minutes...
        if in_range(Utc.from_utc_datetime(&date_time), Duration::minutes(10)) {
            if impedance != 0.0 {
                let muscle_kg = p.get_muscle(weight, impedance);
                let m = Measurement {
                    date_time,
                    weight,
                    bmi: p.get_bmi(weight),
                    water_rate: p.get_water(weight, impedance),
                    bmr: p.get_bmr(weight),
                    visceral_fat: p.get_visceral_fat(weight),
                    bf: p.get_body_fat(weight, impedance),
                    muscle_kg,
                    muscle_rate: (100.0 / weight) * muscle_kg, // convert muscle in kg to percent
                    bone_mass: p.get_bone_mass(weight, impedance),
                };
                debug!("Computed measurement:\n{}", m);

                let mut db_cloned = db.clone();
                match tokio::task::spawn_blocking(move || {
                    info!("🛢️  Storing measurement in the database");
                    if db_cloned.insert_data(m, &p) {
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    } else {
                        Err("Database insert has failed".into())
                    }
                })
                .await
                {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(_) => {
                        return Err("Error spawning DB thread".into());
                    }
                }
            } else {
                return Err("Impedance value is zero".into());
            }
        } else {
            return Err("Error: invalid datetime for mi scale data".into());
        }
    } else {
        return Err("Invalid scale data (eg. not stabilised)".into());
    }
}
