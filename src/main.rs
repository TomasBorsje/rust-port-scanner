pub mod port_map;

use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use crate::port_map::get_port_description;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// IP address to scan
    #[arg(short, long)]
    ip: IpAddr
}

struct PortStatus {
    port: u16,
    listening: bool,
}

const MAX_PORT: u16 = u16::MAX;
const MAX_PERMITS: usize = 500;
static PERMITS: Semaphore = Semaphore::const_new(MAX_PERMITS);

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let ip: IpAddr = args.ip;
    let (tx, mut rx) = channel::<PortStatus>(MAX_PERMITS);

    tokio::spawn(async move {
        for port in 0..MAX_PORT {
            tokio::spawn(process_port(tx.clone(), ip, port));
        }
    });

    while let Some(status) = rx.recv().await {
        if status.listening {
            println!("{0} LISTENING - {1}", status.port, get_port_description(status.port))
        }
    }
}

const TIMEOUT_DURATION: Duration = Duration::from_secs(1);
async fn process_port(tx: Sender<PortStatus>, ip: IpAddr, port: u16) {
    let _permit = PERMITS.acquire().await.expect("Couldn't get semaphore permit");
    let addr = SocketAddr::new(ip, port);

    match timeout(TIMEOUT_DURATION, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => tx.send(PortStatus {
            port,
            listening: true,
        }).await.expect("Couldn't send port status"),
        _ => tx.send(PortStatus {
            port,
            listening: false,
        }).await.expect("Couldn't send port status")
    }
}
