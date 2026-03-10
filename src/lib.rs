use serde::{Deserialize, Serialize};
use std::io::Write;

const SPEED_MAX: u16 = 0x0BB8;

pub fn print_buf(buf: &Vec<u8>) {
    for i in buf {
        print!("0x{:02X?} ", i);
    }
    println!();
}

pub trait DriverReq {
    const OP_CODE: u8;

    fn write_args(&self, buf: &mut Vec<u8>);

    fn make_req(&self, addr: u8) -> Vec<u8> {
        let mut buf = vec![addr, Self::OP_CODE];

        self.write_args(&mut buf);

        buf.push(0x6B);
        buf
    }
}

macro_rules! def_driver_req {
    ($req:tt, $op_code:literal, $aux_code: literal ) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
        pub struct $req;

        impl DriverReq for $req {
            const OP_CODE: u8 = $op_code;
            fn write_args(&self, buf: &mut Vec<u8>) {
                buf.push($aux_code);
            }
        }
    };
}

def_driver_req!(TriggerCalibration, 0x06, 0x45);
def_driver_req!(RebootMotor, 0x08, 0x97);
def_driver_req!(ResetZero, 0x0A, 0x6D);
def_driver_req!(ResetProtection, 0x0E, 0x52);
def_driver_req!(ResetFactory, 0x0F, 0x5F);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnableMotor {
    pub enable: bool,
    pub buffer: bool,
}

impl DriverReq for EnableMotor {
    const OP_CODE: u8 = 0xF3;
    fn write_args(&self, buf: &mut Vec<u8>) {
        buf.push(0xAB);
        buf.push(self.enable as u8);
        buf.push(self.buffer as u8);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ControlSpeed {
    pub cw: bool,
    pub speed: u16,
    pub accel: u8,
    pub buffer: bool,
}

impl DriverReq for ControlSpeed {
    const OP_CODE: u8 = 0xF6;
    fn write_args(&self, buf: &mut Vec<u8>) {
        buf.push(self.cw as u8);

        buf.write(&self.speed.min(SPEED_MAX).to_be_bytes()).unwrap();

        buf.push(self.accel);

        buf.push(self.buffer as u8);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum MotionMode {
    LastTarget = 0,
    Absolute = 1,
    Relative = 2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ControlPosition {
    pub cw: bool,
    pub speed: u16,
    pub accel: u8,
    pub pos: u32,
    pub mode: MotionMode,
    pub buffer: bool,
}

impl DriverReq for ControlPosition {
    const OP_CODE: u8 = 0xFD;
    fn write_args(&self, buf: &mut Vec<u8>) {
        buf.push(self.cw as u8);

        buf.write(&self.speed.min(SPEED_MAX).to_be_bytes()).unwrap();

        buf.push(self.accel);

        buf.write(&self.pos.to_be_bytes()).unwrap();

        buf.push(self.mode as u8);

        buf.push(self.buffer as u8);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HaltMotor {
    pub buffer: bool,
}

impl DriverReq for HaltMotor {
    const OP_CODE: u8 = 0xFE;
    fn write_args(&self, buf: &mut Vec<u8>) {
        buf.write(&[0x98, self.buffer as u8]).unwrap();
    }
}

def_driver_req!(TriggerMotion, 0xFF, 0x66);

pub fn trigger_motion_boardcast() -> Vec<u8> {
    TriggerMotion.make_req(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main() {
        println!("Serial Port List :");
        let ports = serialport::available_ports().expect("No ports found!");
        for p in ports {
            println!("{:<5} - {:?}", p.port_name, p.port_type);
        }

        let mut port = serialport::new("COM4", 115_200)
            .timeout(std::time::Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

        println!("Connected.");

        // let pos =

        let req = ControlPosition {
            cw: true,
            speed: 0x0020,
            accel: 0x2,
            pos: 15_000,
            mode: MotionMode::LastTarget,
            buffer: false,
        }
        .make_req(2);

        // let req = ControlSpeed {
        //     cw: true,
        //     speed: 0x00,
        //     accel: 0x2,
        //     buffer: false,
        // }
        // .make_req(2);

        // let req = EnableMotor {
        //     enable: true,
        //     buffer: false,
        // }
        // .make_req(2);

        loop {
            print_buf(&req);
            port.write(&req).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15_000));
        }
    }
}
