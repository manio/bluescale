use crate::body::Measurement;
use crate::body::Person;
use postgres::{Client, NoTls};
use simplelog::*;

#[derive(Clone)]
pub struct Database {
    pub name: String,
    pub host: String,
    pub dbname: String,
    pub username: String,
    pub password: String,
}

impl Database {
    pub fn insert_data(&mut self, m: Measurement, p: &Person) -> bool {
        let connectionstring = format!(
            "postgres://{}:{}@{}/{}",
            self.username, self.password, self.host, self.dbname
        );
        let client = Client::connect(&connectionstring, NoTls);
        match client {
            Ok(mut client) => {
                 if let Err(e) = client.execute(
                     "INSERT INTO mifit (time, weight, height, bmi, fat_rate, body_water_rate, bone_mass, metabolism, muscle_rate, visceral_fat)
                                 VALUES ($1::timestamp AT time zone 'UTC', $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                     &[&m.date_time, &(m.weight as f64), &(p.height as f64), &(m.bmi as f64), &(m.bf as f64), &(m.water_rate as f64), &(m.bone_mass as f64), &(m.bmr as f64), &(m.muscle_rate as f64), &(m.visceral_fat as f64)],
                 ) {
                     error!("{}: error inserting: {:?}", self.name, e);
                     false
                 } else {
                     true
                 }
            },
            Err(e) => {
                error!("{}: error connecting: {:?}", self.name, e);
                false
            }
        }
    }
}
