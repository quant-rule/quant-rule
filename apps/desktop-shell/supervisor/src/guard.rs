//! The watchdog around one service process.
//!
//! The supervisor runs `qr-desktop --guard` as the parent of every sidecar.
//! The guard connects to the supervisor's control socket before it starts the
//! service, then polls that connection. When the supervisor stops or is killed,
//! the socket closes and the guard terminates the service and exits. No process
//! inspection, no platform process API, and no unsafe signal handling: the
//! kernel closes the socket, and the guard does the rest.
//!
//! The service binary is unmodified. #693/#694 may later move this watch
//! in-process; until then the guard is what makes a crashed supervisor recover
//! on the next start instead of leaving six processes behind.

use std::io::Read;
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::GuardArgs;

const CONTROL_RETRY_MS: u64 = 100;
const CONTROL_TIMEOUT_MS: u64 = 10_000;
const POLL_MS: u64 = 100;

/// Run the guard and return its exit code.
///
/// The guard exits with the service's code when the service exits on its own,
/// and with `0` when it ends the service because the control socket closed.
pub fn run(args: GuardArgs) -> i32 {
    let Some(mut stream) = connect(&args.control) else {
        eprintln!(
            "qr-desktop guard: no control socket at {}; refusing to start {:?}",
            args.control, args.program
        );
        return 1;
    };
    if let Err(error) = stream.set_nonblocking(true) {
        eprintln!("qr-desktop guard: {error}");
        return 1;
    }
    let mut service = Command::new(&args.program);
    service.args(&args.args);
    crate::console::hide_console(&mut service);
    let mut child = match service.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("qr-desktop guard: cannot start {:?}: {error}", args.program);
            return 1;
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(1),
            Ok(None) => {}
            Err(error) => {
                eprintln!(
                    "qr-desktop guard: cannot observe {:?}: {error}",
                    args.program
                );
                let _ = child.kill();
                let _ = child.wait();
                return 1;
            }
        }
        let mut buffer = [0_u8; 64];
        match stream.read(&mut buffer) {
            Ok(0) => {
                let _ = child.kill();
                let _ = child.wait();
                return 0;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return 0;
            }
        }
        thread::sleep(Duration::from_millis(POLL_MS));
    }
}

fn connect(address: &str) -> Option<TcpStream> {
    let timeout = Duration::from_millis(CONTROL_TIMEOUT_MS);
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(stream) = TcpStream::connect(address) {
            return Some(stream);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(CONTROL_RETRY_MS));
    }
}
