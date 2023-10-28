mod service;
use std::{
    thread::sleep,
    time::{Duration, Instant},
    ffi::OsString,
    env
};
use std::fs::File;
use log::{error, info, LevelFilter};
use simplelog::*;
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use clap::Command;

const DEFAULT_PROGRAM: &str = "kodi.exe";
const DEFAULT_RUN_PATH: &str = "C:\\Program Files\\Kodi\\kodi.exe";
const DEFAULT_HOST_IP: &str = "192.168.1.132";
const DEFAULT_LOG_LEVEL: &str = "warn";
fn main() -> windows_service::Result<()> {
    // Parse args
    let log_file = File::create("wol_service.log").unwrap();
    let cmd = Command::new("WakeOnLan Start")
        .subcommand(
            Command::new("install")
                .about("Installs the service")
                .arg(
                    clap::arg!(--"program" <PROGRAM>)
                        .help("Program to use")
                        .default_value(DEFAULT_PROGRAM),
                )
                .arg(
                    clap::arg!(--"run-path" <RUN_PATH>)
                        .help("Path to run the program from")
                        .default_value(DEFAULT_RUN_PATH),
                )
                .arg(
                    clap::arg!(--"host-ip" <HOST_IP>)
                        .help("IP address of the host")
                        .default_value(DEFAULT_HOST_IP),
                )
                .arg(
                    clap::arg!(--"log-level" <LOG_LEVEL>)
                        .help("Logging level")
                        .default_value(DEFAULT_LOG_LEVEL),
                ),
            )
        .subcommand(Command::new("uninstall").about("Uninstalls the service")
                        .arg(
                            clap::arg!(--"log-level" <LOG_LEVEL>)
                                .help("Logging level")
                                .default_value(DEFAULT_LOG_LEVEL),
                        ),
        )
        .allow_external_subcommands(true);

    // Initialize logging
    let matches = cmd.get_matches();
    // Extract log level early, and set a default if it doesn't exist
    let log_level = if let Some(matches) = matches.subcommand_matches("install")
        .or_else(|| matches.subcommand_matches("uninstall")) {
        match matches.get_one::<String>("log-level").unwrap_or(&String::from("info")).as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "warn"=> LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    } else {
        LevelFilter::Info // Default level if no subcommand or something goes wrong
    };
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(log_level, Config::default(), log_file)
        ]
    ).unwrap();
    info!("{:?}", matches.subcommand());
    match matches.subcommand() {
        Some(("install", install_matches)) => {
            info!("Install...");
            let program = install_matches.get_one::<String>("program").unwrap();
            let run_path = install_matches.get_one::<String>("run-path").unwrap();
            let host_ip = install_matches.get_one::<String>("host-ip").unwrap();
            install(&program, &run_path, &host_ip,
                    &install_matches.get_one::<String>("log-level").unwrap())?;
        }
        Some(("uninstall", _)) => {
            info!("Uninstall...");
            uninstall()?;
        }
        Some(_) => {
            info!("Run service");
            service::run()?;
        }
        None => {
            error!("No args passed");
        }
    }

    Ok(())
}

fn install(prg: &String, run_path: &String, host_ip: &String, log_level: &String) -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = env::current_exe()
        .unwrap()
        .with_file_name("WolStart.exe");

    let service_info = ServiceInfo {
        name: OsString::from("wol_service"),
        display_name: OsString::from("WakeOnLan service"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        executable_path: service_binary_path,
        error_control: ServiceErrorControl::Normal,
        launch_arguments: vec![
            OsString::from(prg),
            OsString::from(run_path),
            OsString::from(host_ip),
            OsString::from(log_level),
        ],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("Windows service to run program on receiving wake on lan packet")?;
    Ok(())
}

fn uninstall() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service("wol_service", service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(15);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service("wol_service", ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("wol_service is deleted.");
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("wol_service is marked for deletion.");

    Ok(())
}