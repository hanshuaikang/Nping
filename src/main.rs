// 引入自定义模块
mod network;
mod ui;
mod terminal;
mod ip_data;
use clap::Parser;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::task;
use crate::ip_data::IpData;
use crate::network::send_ping;


#[derive(Parser, Debug)]
#[command(
    version = "v0.1.0",
    author = "hanshuaikang<https://github.com/hanshuaikang>",
    about = "Nping with concurrent,chart,multiple addresses,real-time data update"
)]
struct Args {
    /// Target IP address or hostname to ping
    #[arg(help = "target IP address or hostname to ping", required = true)]
    target: Vec<String>,

    /// Number of pings to send, when count is 0, the maximum number of pings per address is calculated
    #[arg(short, long, default_value_t = 0, help = "Number of pings to send")]
    count: usize,

    /// Interval in seconds between pings
    #[arg(short, long, default_value_t = 0, help = "Interval in seconds between pings")]
    interval: i32,

    #[arg(short, long, default_value_t = 64, help = "Packet size")]
    size: i32,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse command line arguments
    let args = Args::parse();

    // init terminal
    ui::init_terminal()?;

    // set Ctrl+C handler
    let running = Arc::new(Mutex::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            let mut running = running.lock().unwrap();
            *running = false;
        })
            .expect("cat not set Ctrl+C handler");
    }


    let targets: Vec<String> = args.target.into_iter().collect::<HashSet<_>>().into_iter().collect();

    let mut count = args.count;
    if args.count == 0 {
        count = network::calculate_max_pings_per_address(targets.len());
    }

    // check if the number of pings exceeds the maximum number of pings
    if args.count > 0 && targets.len() * args.count > network::MAX_PINGS {
        let max_count = network::calculate_max_pings_per_address(targets.len());
        eprintln!("when you ping {} address, the maximum number of pings is {}", targets.len(), max_count);
        std::process::exit(1);
    }


    let res = run_app(targets, count, args.interval, args.size, running.clone()).await;

    // if error print error message and exit
    if let Err(err) = res {
        eprintln!("{}", err);
        std::process::exit(1);
    }
    Ok(())
}

async fn run_app(
    targets: Vec<String>,
    count: usize,
    interval: i32,
    size: i32,
    running: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {

    // Create terminal instance
    let terminal = ui::init_terminal().unwrap();
    let terminal_guard = Arc::new(Mutex::new(terminal::TerminalGuard::new(terminal)));

    // Define statistics variables
    let ip_data = Arc::new(Mutex::new(targets.iter().map(|target| IpData {
        ip: String::from(""),
        addr: target.to_string(),
        rtts: VecDeque::new(),
        last_attr: 0.0,
        min_rtt: 0.0,
        max_rtt: 0.0,
        sent: 0,
        received: 0,
        pop_count: 0,
    }).collect::<Vec<_>>()));

    // Resolve target addresses
    let mut addrs = Vec::new();
    for target in &targets {
        let addr = network::resolve_target(target)?;
        addrs.push(addr);
    }

    let (tx, rx) = network::init_transport_channel()?;
    let tx = Arc::new(Mutex::new(tx));
    let rx = Arc::new(Mutex::new(rx));

    let interval = if interval == 0 { 500 } else { interval * 1000 };
    let mut tasks = Vec::new();
    for (i, addr) in addrs.iter().enumerate() {
        let ip_data = ip_data.clone();
        let addr = *addr;
        let terminal_guard = terminal_guard.clone();
        let running = running.clone();
        let tx_clone = tx.clone();
        let rx_clone = rx.clone();
        let task = task::spawn({
            let ip_data = ip_data.clone();
            async move {
                send_ping(addr, i, count, interval, size, ip_data.clone(), move || {
                    let mut terminal_guard = terminal_guard.lock().unwrap();
                    ui::draw_interface(&mut terminal_guard.terminal.as_mut().unwrap(), &ip_data.lock().unwrap()).unwrap();
                }, running.clone(), tx_clone, rx_clone).await.unwrap();
            }
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}