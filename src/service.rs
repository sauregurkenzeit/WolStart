mod run_as_current_user;
use pnet::datalink::{self, NetworkInterface};
use sysinfo::{System, SystemExt};
use log::{error, info, warn, debug};
use std::{sync::{
    Arc, Mutex, mpsc::{self, TryRecvError, Receiver}
    }, ffi::OsString, time::Duration, env};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher, Result,
};

const SERVICE_NAME: &str = "wol_service";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
pub fn run() -> Result<()> {
    info!("Starting the service...");
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(_) => {
            info!("Service stopped successfully.");
            Ok(())
        },
        Err(e) => {
            error!("Service stopped with error: {:?}", e);
            Err(e)
        }
    }
}

// Generate the windows service boilerplate.
// The boilerplate contains the low-level service entry function (ffi_service_main) that parses
// incoming service arguments into Vec<OsString> and passes them to user defined service
// entry (wol_service_main).
define_windows_service!(ffi_service_main, wol_service_main);

pub fn wol_service_main(_: Vec<OsString>) {
    let arguments: Vec<String> = env::args().collect();
    // Create a channel to be able to poll a stop event from the service worker loop.
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let shutdown_rx = Arc::new(Mutex::new(shutdown_rx));

    // Define system service event handler that will be receiving service events.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle: Option<ServiceStatusHandle> =
        match service_control_handler::register(SERVICE_NAME, event_handler) {
            Ok(handle) => Some(handle),
            Err(e) => {
                error!("Failed to register the service control handler: {:?}", e);
                None
            }
        };

    // Tell the system that service is running
    set_status(&status_handle, "Service status set to RUNNING", ServiceState::Running, 0);

    debug!("Service entry function called with arguments: {:?}", arguments);

    // Ensure we have at least 4 arguments.
    if arguments.len() < 4 {
        set_status(&status_handle, "Insufficient arguments provided to the service", ServiceState::Stopped, 1);
        return;
    }

    // Skipping arguments[0] as it's the service name.
    let prg = arguments[1].as_str();
    let run_path = arguments[2].as_str();
    let host_ip = arguments[3].as_str();

    if let Err(e) = run_service(prg, run_path, host_ip,
        shutdown_rx,
        status_handle
    ) {
       set_status(&status_handle, "Error running the service: {e}", ServiceState::Stopped, 1);
    }
}

pub fn run_service(prg: &str, run_path: &str, host_ip: &str,
                   shutdown_rx:Arc<Mutex<Receiver<()>>>,
                   status_handle: Option<ServiceStatusHandle>) -> Result<()> {
        let sleep_duration = std::time::Duration::from_secs(1);
        let mut sleep_counter = 0;
        let inner_shutdown_rx = Arc::clone(&shutdown_rx);
        let interfaces = datalink::interfaces();
        let interface = match interfaces.into_iter().find(|iface| {
            iface.ips.iter().any(|ip| ip.to_string().starts_with(host_ip))
        }) {
            Some(found_interface) => {
                info!("Found an interface with target IP address starting with: {}", host_ip);
                found_interface
            },
            None => {
                set_status(&status_handle, "Could not find the interface with IP address starting with: {host_ip}", ServiceState::Stopped, 1);
                panic!("Could not find the interface with IP address starting with: {}", host_ip)
            }
        };

        loop {
            if stop_signal_handler(&shutdown_rx) {
                debug!("Received STOP signal in outer loop");
                break;
            }
            if !is_program_running(prg) {
                info!("{} not running; start listening for WOL packet", prg);
                if listen_for_wol(&interface, run_path, inner_shutdown_rx.clone(), &status_handle) {
                    break;
                }
            }

            // Sleep for 1 second, and then check the stop signal.
            // This way, the maximum delay to handle the stop signal is 1 second.
            std::thread::sleep(sleep_duration);
            sleep_counter += 1;

            if sleep_counter >= 10 {
                sleep_counter = 0;
            }
        }
        // Tell the system that service has stopped.
        set_status(&status_handle, "Service stopped...", ServiceState::Stopped, 0);
        Ok(())
}

fn is_program_running(prg: &str) -> bool {
    let sys = System::new_all();
    let x = !sys.processes_by_name(prg).next().is_none();
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

fn listen_for_wol(interface: &NetworkInterface, run_path: &str,
                  shutdown_rx: Arc<Mutex<Receiver<()>>>, status_handle: &Option<ServiceStatusHandle>) -> bool{
    let channel = datalink::channel(interface, Default::default()).unwrap();

    let mut rx = match channel {
        datalink::Channel::Ethernet(_, rx) => {
            debug!("Datalink channel created");
            rx
        },
        _ => {
            set_status(&status_handle,"Failed to create datalink channel", ServiceState::Stopped, 1);
            panic!("Failed to create datalink channel")
        }
    };

    loop {
        if stop_signal_handler(&shutdown_rx){
            debug!("Receive STOP signal in inner loop");
            return true
        }
        match rx.next() {
            Ok(packet) => {
                if is_wol_packet(packet) {
                    info!("Wake-on-LAN packet detected!");
                    // Stop listening and break the loop.
                    break;
                }
            },
            Err(e) => {
                warn!("An error occurred while reading packet: {:?}", e);
                continue;
            },
        }
    }

    match run_as_current_user::start_process_as_current_user(run_path,
                                                             Some(""),
                                                             run_path.split("\\").next(),
                                                             true){
        Ok(result) => {
            info!("Successfully started the command with process id: {:?}", result);
            false
        },
        Err(e) => {
            error!("Failed to start {}. Error: {:?}", run_path, e);
            false
        }
    }
}

fn set_status(status_handle: &Option<ServiceStatusHandle>, message: &str, state: ServiceState, exit: u32) {
    if let Some(handle) = status_handle { // Use pattern matching here
        let control_accepted = match state {
            ServiceState::Stopped => ServiceControlAccept::empty(),
            _ => ServiceControlAccept::STOP,
        };
        let status = ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: state,
            controls_accepted: control_accepted,
            exit_code: ServiceExitCode::Win32(exit),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };
        match handle.set_service_status(status) { // Use the handle here
            Ok(_) => {
                match exit {
                    0 => { debug!("{}", message); },
                    _ => { error!("{}", message); }
                }
            },
            Err(e) => {
                error!("Failed to set service status. Error: {:?}", e);
            }
        }
    }
}

fn stop_signal_handler(shutdown_rx: &Arc<Mutex<Receiver<()>>>) -> bool {
    match shutdown_rx.lock().unwrap().try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {
            debug!("Receive STOP signal");
            true
        },
        Err(TryRecvError::Empty) => false,
    }
}