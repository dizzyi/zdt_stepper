use std::io::Write;
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

    let mut port = serialport::new(port, 115_200)
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

    // port.write(&req).unwrap();

    loop {
        print_buf(&req);
        port.write(&req).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15_000));
    }
}
