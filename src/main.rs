extern crate pnet;
extern crate sysinfo;

use pnet::packet::Packet;
use pnet::datalink::{self, NetworkInterface};
use sysinfo::{System, SystemExt};
use std::process::Command;

const PROGRAM: &str = "kodi.exe";
const RUN_PATH: &str = "C:\\Program Files\\Kodi\\kodi.exe";
const HOST_IP: &str = "192.168.1.132";


fn main() {
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter().find(|iface| iface.ips.iter()
        .any(|ip| ip.to_string().starts_with(HOST_IP)))
    .expect("Could not find the interface with the target IP address");

    loop {
        // listen_for_wol(&interface)
        if !is_kodi_running() {
            listen_for_wol(&interface);
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

fn is_kodi_running() -> bool {
    let sys = System::new_all();
    let x = !sys.processes_by_name(PROGRAM).next().is_none();
    x
}

fn is_wol_packet(packet: &[u8]) -> bool {
    // Minimum length for WOL payload
    if packet.len() < 6 + 16 * 6 {
        return false;
    }

    let wol_start = packet.len() - (6 + 16 * 6);

    // Check for 6 bytes of 0xFF
    if packet[wol_start..wol_start + 6] != [0xff, 0xff, 0xff, 0xff, 0xff, 0xff] {
        return false;
    }

    // Get the repeated MAC address from the packet
    let mac = &packet[wol_start + 6..wol_start + 12];

    // Check for 16 repetitions of the MAC address
    for i in 0..16 {
        if packet[wol_start + 6 + i * 6..wol_start + 6 + (i + 1) * 6] != *mac {
            return false;
        }
    }

    true
}

fn listen_for_wol(interface: &NetworkInterface) {
    let channel = datalink::channel(interface, Default::default()).unwrap();

    let mut rx = match channel {
        datalink::Channel::Ethernet(_, rx) => rx,
        _ => panic!("Failed to create datalink channel"),
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                if is_wol_packet(packet) {
                    println!("Wake-on-LAN packet detected!");
                    // Stop listening and break the loop.
                    break;
                }
            },
            Err(e) => {
                eprintln!("An error occurred while reading packet: {:?}", e);
                continue;
            },
        }
    }

    Command::new(RUN_PATH).spawn().expect("Failed to start Kodi");
}
