use std::{fs::read, io::Write};
use zdt_stepper::*;

use serialport;

fn main() {
    dotenvy::dotenv().unwrap();

    println!("Serial Port List :");
    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports {
        println!("{:<5} - {:?}", p.port_name, p.port_type);
    }

    let port = std::env::var("PORT").unwrap();
    let arduino_port = std::env::var("ARDUINO_PORT").unwrap();

    let mut port = serialport::new(port, 115_200)
        .timeout(std::time::Duration::from_millis(1000))
        .open()
        .expect("Failed to open port");
    let mut arduino_port = serialport::new(arduino_port, 115_200)
        .timeout(std::time::Duration::from_millis(1000))
        .open()
        .expect("Failed to open port");

    println!("Connected.");

    std::thread::sleep(std::time::Duration::from_secs(3));

    let state = std::sync::Arc::new(std::sync::RwLock::new(false));

    {
        let state = state.clone();
        std::thread::spawn(move || {
            let mut buf = String::new();
            let mut last_state = false;
            loop {
                buf.clear();

                loop {
                    let mut b = [0];
                    arduino_port.read(&mut b).unwrap();
                    if b[0] == b'\n' {
                        break;
                    }

                    buf.push(b[0] as char);
                }

                // println!("{}", buf);

                let v = buf.split(",").collect::<Vec<_>>();

                if let Some(v) = v.get(2) {
                    // let state = v.contains("1");
                    let read_state = v.contains("1");
                    if last_state != read_state {
                        println!("state change: {} -> {}", last_state, read_state);
                    }

                    last_state = read_state;
                    // println!("{}", read_state);
                    *state.write().unwrap() = read_state;
                } else {
                    println!("WARN PARSE ERROR: {:?}", buf);
                }

                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
    }

    // return;

    // let pos =

    let spin_req = ControlPosition {
        cw: false,
        speed: 0x0020,
        accel: 0x2,
        pos: 60_000,
        mode: MotionMode::LastTarget,
        buffer: false,
    }
    .make_req(2);

    let halt_req = ControlSpeed {
        cw: true,
        speed: 0x00,
        accel: 0x2,
        buffer: false,
    }
    .make_req(2);

    // let req = EnableMotor {
    //     enable: true,
    //     buffer: false,
    // }
    // .make_req(2);

    // port.write(&req).unwrap();

    loop {
        // print_buf(&req);
        println!("> start spin");
        port.write(&spin_req).unwrap();
        let dt = std::time::Instant::now();
        loop {
            let elapsed = dt.elapsed().as_millis();

            if elapsed > 20_000 {
                println!("> timeout");
                break;
            }

            if elapsed > 2_000 && !*state.read().unwrap() {
                println!("> sensor reached");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        println!("> stopping");
        port.write(&halt_req).unwrap();

        println!("> sleeping");
        println!("> ");
        std::thread::sleep(std::time::Duration::from_millis(5_000));
    }
}
