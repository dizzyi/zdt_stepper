#[macro_use]
extern crate rocket;

use rocket::{build, fs::NamedFile};
use std::{fs::read, io::Write};
use zdt_stepper::*;

use serialport::{self, SerialPort};

use tokio::sync::{broadcast, mpsc, watch};

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

    let (command_tx, command_rx) = mpsc::channel(256);
    let (enable_tx, enable_rx) = mpsc::channel(256);
    let (state_tx, state_rx) = watch::channel(0);

    std::thread::sleep(std::time::Duration::from_secs(3));
    std::thread::spawn(move || controller(port, arduino_port, command_rx, enable_rx, state_tx));

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            rocket::build()
                .mount("/", routes![ping, index, state, spin, enable])
                .manage(MyConfig {
                    command_tx,
                    enable_tx,
                    state_rx,
                })
                .launch()
                .await
        });
}

pub struct MyConfig {
    command_tx: mpsc::Sender<bool>,
    enable_tx: mpsc::Sender<bool>,
    state_rx: watch::Receiver<u8>,
}

#[get("/ping")]
fn ping() -> String {
    format!("pong")
}

#[get("/")]
async fn index() -> Option<NamedFile> {
    NamedFile::open("./asset/index.html").await.ok()
}

#[get("/state")]
async fn state(my_config: &rocket::State<MyConfig>) -> String {
    my_config.state_rx.borrow().to_string()
}

#[get("/spin/<cw>")]
async fn spin(cw: &str, my_config: &rocket::State<MyConfig>) -> String {
    if *my_config.state_rx.borrow() != 1 {
        return "Busy".to_string();
    }
    my_config.command_tx.send(cw.contains("1")).await;
    "OK".to_string()
}

#[get("/enable/<b>")]
async fn enable(b: &str, my_config: &rocket::State<MyConfig>) -> String {
    let s = *my_config.state_rx.borrow();
    if s == 2 {
        return "Busy".to_string();
    }
    my_config.enable_tx.send(b.contains("1")).await;
    "OK".to_string()
}

fn controller(
    mut port: Box<dyn SerialPort>,
    mut arduino_port: Box<dyn SerialPort>,
    mut command_rx: mpsc::Receiver<bool>,
    mut enable_rx: mpsc::Receiver<bool>,
    mut state_tx: watch::Sender<u8>,
) {
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
    let mut ctr_pos = ControlPosition {
        cw: false,
        speed: 0x0028,
        accel: 0x2,
        pos: 60_000,
        mode: MotionMode::LastTarget,
        buffer: false,
    };

    let mut spin_req = ctr_pos.clone().make_req(2);

    let halt_req = ControlSpeed {
        cw: true,
        speed: 0x00,
        accel: 0x8,
        buffer: false,
    }
    .make_req(2);

    // let req = EnableMotor {
    //     enable: true,
    //     buffer: false,
    // }
    // .make_req(2);

    // port.write(&req).unwrap();

    let mut enabled = true;

    loop {
        if !enabled {
            let Ok(enable_cmd) = enable_rx.try_recv() else {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            };
            println!("read enable command: {}", enable_cmd);

            if !enable_cmd {
                continue;
            }

            port.write(
                &EnableMotor {
                    enable: true,
                    buffer: false,
                }
                .make_req(2),
            )
            .unwrap();
            enabled = true;
            state_tx.send(1);

            continue;
        } else {
            if let Ok(enable_cmd) = enable_rx.try_recv() {
                std::thread::sleep(std::time::Duration::from_millis(5));
                println!("read enable command: {}", enable_cmd);

                if enable_cmd {
                    continue;
                }

                port.write(
                    &EnableMotor {
                        enable: false,
                        buffer: false,
                    }
                    .make_req(2),
                )
                .unwrap();
                enabled = false;
                state_tx.send(0);
                continue;
            };
        }

        state_tx.send(1);
        println!("listening for command");
        // print_buf(&req);
        let Ok(command) = command_rx.try_recv() else {
            // panic!("unexpected error");
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        };

        println!("recv command : {}", command);

        ctr_pos.cw = command;
        spin_req = ctr_pos.clone().make_req(2);

        state_tx.send(2);

        println!("> start spin");
        port.write(&spin_req).unwrap();
        let dt = std::time::Instant::now();
        loop {
            let elapsed = dt.elapsed().as_millis();

            if elapsed > 20_000 {
                println!("> timeout");
                break;
            }

            if elapsed > 3_000 && !*state.read().unwrap() {
                println!("> sensor reached");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        println!("> stopping");
        port.write(&halt_req).unwrap();
        state_tx.send(1);

        // println!("> sleeping");
        // println!("> ");
        // std::thread::sleep(std::time::Duration::from_millis(5_000));
    }
}
