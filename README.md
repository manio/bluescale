## About
This is a project for reading *Xiaomi Mi Body Composition Scale 2*.<br>
It's called *bluescale* because of using bluetooth ([bluer](https://crates.io/crates/bluer)) and reading a scale :)<br>
It is a [Rust](https://www.rust-lang.org/) command-line project intended to run under linux.<br>
The purpose is to read the scale using the bluetooth and save the measurements directly to a [PostgreSQL](https://www.postgresql.org/) database.<br>

## Origins
I started my work with this simple python script:<br>
https://github.com/srijansaxena11/miscale<br>
but the measurements was not accurate, so I head to this great _openScale_ project:<br>
https://github.com/oliexdev/openScale/<br>
The calculations/formulas used in openScale *matches* the former `MiFit` and currently `Zepp Life` android app, and this was important for me.<br>
I converted the code from java to rust and used here in this project.<br>
Lastly I added the `Basal metabolism` calculation from here:<br>
https://github.com/zibous/ha-miscale2/blob/master/lib/body_metrics.py#L58

## Internals
The scale is a low Bluetooth Low Energy (BLE) device. When a user steps on it, it is starting the bluetooth communication and it is discoverable. After 20 minutes, it is auto-powered off to save the energy.<br>
It is broadcasting the last measurement in so called `Service UUID`.<br>
The protocol is much more sophisticated: the scale has some buffer of historical data, which can be obtained and marked as read. There is also a way to set the date/time of the scale.<br>
Regarding the timestamp: It is checked if the last measurement's timestamp is in range of 10 minutes from current time, if this is true, than it is treated as correct reading.<br>
I can see that at least two projects just reads the last measurements only using `Service UUID` (opposite to the _openScale_ way, which is reading scale's buffer with measurements data), so I did this the same way.

## Usage
Start the program, it is constantly monitoring for specifed MAC address of the scale (if you don't provide the MAC it will work and will read from all scales in range). When it is available, then it sleeps for a while and tries to read the last stable measurement. If it is complete (with impedance data), then it is computing the body composition and write the data out to a postgres table. This program can run forever, making the daily/weekly measurements very easy.<br>
There is also a nice feature which uses the `PC speaker` beeps for notifications. When the scale is discovered, it beeps, and when the record is successfully saved to database it also beeps differently signalling that all is fine and we have the data :)

## Config
The project uses a simple configuration file:<br>
`/etc/bluescale.conf`<br>

A sample file may have the following contents:<br>
```
[miscale]
mac = 00:00:00:00:00:00  #enter your scale MAC here, or comment it out

[profile]
sex = 1  #1=male, 0=female
birthday = 2000-01-01
height = 180

[postgres]
host=192.168.1.1
dbname=database_name
username=database_user
password=database_password
```
