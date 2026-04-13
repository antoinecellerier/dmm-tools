//! Send the GetName command and print the raw response.

use dmm_lib::cp2110::{Cp2110, PID, VID};
use dmm_lib::protocol::ut61eplus::command::Command;
use dmm_lib::transport::Transport;

fn main() {
    env_logger::init();

    let api = hidapi::HidApi::new().expect("Failed to init HID API");
    let device = api.open(VID, PID).expect("Failed to open device");
    let cp = Cp2110::new(device);
    cp.init_uart().expect("Failed to init UART");

    // Send GetName command
    let cmd = Command::GetName.encode();
    println!("Sending GetName: {:02X?}", cmd);
    cp.write(&cmd).expect("Failed to write");

    // Read response bytes
    let mut buf = Vec::new();
    let mut tmp = [0u8; 64];
    for _ in 0..64 {
        match cp.read_timeout(&mut tmp, 2000) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }
    }

    println!("Response ({} bytes): {:02X?}", buf.len(), buf);
    println!("As ASCII: {:?}", String::from_utf8_lossy(&buf));
}
