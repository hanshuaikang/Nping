mod network;
mod ui;
mod terminal;
mod ip_data;
use clap::Parser;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::task;
use crate::ip_data::IpData;
use std::sync::mpsc;
use std::thread;
use crate::network::send_ping;

#[derive(Parser, Debug)]
#[command(
    version = "v0.2.4",
    author = "hanshuaikang<https://github.com/hanshuaikang>",
    about = "🏎  Nping mean NB Ping, A Ping Tool in Rust with Real-Time Data and Visualizations"
)]
struct Args {
    /// Target IP address or hostname to ping
    #[arg(help = "target IP address or hostname to ping", required = true)]
    target: Vec<String>,

    /// Number of pings to send, when count is 0, the maximum number of pings per address is calculated
    #[arg(short, long, default_value_t = 65535, help = "Number of pings to send")]
    count: usize,

    /// Interval in seconds between pings
    #[arg(short, long, default_value_t = 0, help = "Interval in seconds between pings")]
    interval: i32,

    #[clap(long = "force_ipv6", default_value_t = false, short = '6', help = "Force using IPv6")]
    pub force_ipv6: bool,

    #[arg(
        short = 'm',
        long,
        default_value_t = 0,
        help = "Specify the maximum number of target addresses, Only works on one target address"
    )]
    multiple: i32,

    #[arg(short, long, default_value = "graph", help = "view mode graph or table")]
    view_type: String,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse command line arguments
    let args = Args::parse();

    // set Ctrl+C handler
    let running = Arc::new(Mutex::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            let mut running = running.lock().unwrap();
            *running = false;
        })
            .expect("cannot set Ctrl+C handler");
    }


    let targets: Vec<String> = args.target.into_iter().collect::<HashSet<_>>().into_iter().collect();

    let res = run_app(targets, args.count, args.interval, running.clone(), args.force_ipv6, args.multiple, args.view_type).await;

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
    running: Arc<Mutex<bool>>,
    force_ipv6: bool,
    multiple: i32,
    view_type: String,
) -> Result<(), Box<dyn std::error::Error>> {

    // init terminal
    ui::init_terminal()?;

    // Create terminal instance
    let terminal = ui::init_terminal().unwrap();
    let terminal_guard = Arc::new(Mutex::new(terminal::TerminalGuard::new(terminal)));


    // ip channel
    let (ping_update_tx, ping_update_rx) = mpsc::sync_channel::<IpData>(0);

    let ping_update_tx = Arc::new(ping_update_tx);


    let mut ips = Vec::new();
    // if multiple is set, get multiple IP addresses for each target
    if targets.len() == 1 && multiple > 0 {
        // get multiple IP addresses for the target
        ips = network::get_multiple_host_ipaddr(&targets[0], force_ipv6, multiple as usize)?;
    } else {
        // get IP address for each target
        for target in &targets {
            let ip = network::get_host_ipaddr(target, force_ipv6)?;
            ips.push(ip);
        }
    }

    // Define statistics variables
    let ip_data = Arc::new(Mutex::new(ips.iter().enumerate().map(|(i, _)| IpData {
        ip: String::new(),
        addr: if targets.len() == 1 { targets[0].clone() } else { targets[i].clone() },
        rtts: VecDeque::new(),
        last_attr: 0.0,
        min_rtt: 0.0,
        max_rtt: 0.0,
        timeout: 0,
        received: 0,
        pop_count: 0,
    }).collect::<Vec<_>>()));

    let view_type = Arc::new(view_type);

    let errs = Arc::new(Mutex::new(Vec::new()));

    let interval = if interval == 0 { 500 } else { interval * 1000 };
    let mut tasks = Vec::new();

    {
        let ip_data = ip_data.clone();
        let errs = errs.clone();
        let terminal_guard = terminal_guard.clone();
        let view_type = view_type.clone();

        {
            let mut guard = terminal_guard.lock().unwrap();
            let ip_data = ip_data.lock().unwrap();
            // first draw ui
            ui::draw_interface(
                &mut guard.terminal.as_mut().unwrap(),
                &view_type,
                &ip_data,
                &mut errs.lock().unwrap(),
            ).ok();
        }

        thread::spawn(move || {
            while let Ok(updated_data) = ping_update_rx.recv() {
                let mut ip_data = ip_data.lock().unwrap();
                if let Some(pos) = ip_data.iter().position(|d| d.addr == updated_data.addr && d.ip == updated_data.ip) {
                    ip_data[pos] = updated_data;
                }
                let mut guard = terminal_guard.lock().unwrap();
                ui::draw_interface(
                    &mut guard.terminal.as_mut().unwrap(),
                    &view_type,
                    &ip_data,
                    &mut errs.lock().unwrap(),
                ).ok();
            }
        });
    }
    for (i, ip) in ips.iter().enumerate() {
        let ip = ip.clone();
        let running = running.clone();
        let errs = errs.clone();
        let task = task::spawn({
            let errs = errs.clone();
            let ping_update_tx = ping_update_tx.clone();
            let ip_data = ip_data.clone();
            let mut data = ip_data.lock().unwrap();
            // update the ip
            data[i].ip = ip.clone();
            let addr = data[i].addr.clone();
            async move {
                send_ping(addr, ip, errs.clone(), count, interval, running.clone(), ping_update_tx).await.unwrap();
            }
        });
        tasks.push(task)
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}