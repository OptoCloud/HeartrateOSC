use std::net::{SocketAddr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};
use rosc::{encoder, OscBundle, OscPacket, OscTime};

use crate::heartrate_measurement::HeartRateMeasurement;

fn get_timetag() -> Result<OscTime, Box<dyn std::error::Error>> {
    let ms_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() + 2208988800; // 2208988800 is the number of seconds between 1900 and 1970
    let seconds_since_epoch = ms_since_epoch / 1000;
    let fractional_seconds = ms_since_epoch - (seconds_since_epoch * 1000);

    if seconds_since_epoch > 0xFFFFFFFF {
        return Err("Time is too far in the future".into());
    }

    Ok(OscTime { seconds: seconds_since_epoch as u32, fractional: fractional_seconds as u32 })
}

pub struct VRCOSCClient {
    socket: UdpSocket,
    hr_flip_flop: bool,
}

impl VRCOSCClient {
    pub fn new(addr: &SocketAddr) -> Self {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
        socket.connect(addr).expect("couldn't connect to address");

        VRCOSCClient {
            socket,
            hr_flip_flop: false,
        }
    }

    pub fn send_heartrate(&mut self, heart_rate: &HeartRateMeasurement) -> Result<(), Box<dyn std::error::Error>> {
        self.hr_flip_flop = !self.hr_flip_flop;

        let average_rr_interval: i32;
        if heart_rate.rr_intervals.len() > 0 {
            average_rr_interval = (heart_rate.rr_intervals.iter().sum::<u16>() as usize / heart_rate.rr_intervals.len()) as i32;
        } else {
            // Calculate average RR interval from heart rate
            average_rr_interval = (60.0 / heart_rate.heart_rate as f32 * 1000.0) as i32;
        }

        let message = |path: &str, value| OscPacket::Message(rosc::OscMessage {
            addr: path.to_string(),
            args: vec![value],
        });

        let packet = OscPacket::Bundle(OscBundle {
            timetag: get_timetag()?,
            content: vec![
                message("/avatar/parameters/HR", rosc::OscType::Int(heart_rate.heart_rate as i32)),
                message("/avatar/parameters/onesHR", rosc::OscType::Int((heart_rate.heart_rate % 10) as i32)),
                message("/avatar/parameters/tensHR", rosc::OscType::Int((heart_rate.heart_rate % 100 / 10) as i32)),
                message("/avatar/parameters/hundredsHR", rosc::OscType::Int((heart_rate.heart_rate / 100) as i32)),
                message("/avatar/parameters/floatHR", rosc::OscType::Float(heart_rate.heart_rate as f32 / 255.0 * 2.0 - 1.0)),
                message("/avatar/parameters/isHRBeat", rosc::OscType::Bool(true)),
                message("/avatar/parameters/HeartBeatToggle", rosc::OscType::Bool(self.hr_flip_flop)),
                message("/avatar/parameters/isHRConnected", rosc::OscType::Bool(true)),
                message("/avatar/parameters/RRInterval", rosc::OscType::Int(average_rr_interval)),
            ]
        });

        let bytes = encoder::encode(&packet)?;

        self.socket.send(&bytes)?;

        Ok(())
    }
}