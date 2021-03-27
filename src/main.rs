use std::{io, thread, str};
use serialport::{SerialPort};

fn main() {
    let port_name = "/dev/cu.usbserial-DN05IJFB";
    let port = serialport::new(port_name, 345600)
        .open()
        .expect("Failed to open serial port");

    let listen_port_thread = thread::spawn(|| {
        listen_port(port);
    });

    listen_port_thread.join().unwrap();
}

fn listen_port(mut port: Box<dyn SerialPort>) {
    let mut buffer: [u8; 1] = [0; 1];
    let mut bytes = Vec::new();
    loop {
        match port.read(&mut buffer) {
            Ok(_length) => {
                bytes.push(buffer[0]);
                if bytes[bytes.len() - 1] == 13 {
                    println!("{:?}", str::from_utf8(&bytes));
                    bytes = Vec::new();
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}