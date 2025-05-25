mod network;
mod draw;
mod terminal;
mod ip_data;
mod ui;

use clap::Parser;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::task;
use crate::ip_data::IpData;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
use crate::network::send_ping;

#[derive(Parser, Debug)]
#[command(
    version = "v0.3.1",
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

    #[arg(short, long, default_value = "graph", help = "View mode graph/table/point")]
    view_type: String,

    #[arg(short = 'o', long = "output", help = "Output file to save ping results")]
    output: Option<String>,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse command line arguments
    let args = Args::parse();

    // set Ctrl+C and q and esc to exit
    let running = Arc::new(Mutex::new(true));
    {
        let running = running.clone();
        thread::spawn(move || {
            loop {
                // if running is false, exit the loop
                if !*running.lock().unwrap() {
                    break;
                }

                if let Ok(true) = event::poll(Duration::from_millis(50)) {
                    if let Ok(Event::Key(key)) = event::read() {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                *running.lock().unwrap() = false;
                                break;
                            },
                            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                                *running.lock().unwrap() = false;
                                break;
                            },
                            _ => {}
                        }
                    }
                }
            }
        });
    }

    // check output file
    if let Some(ref output_path) = args.output {
        if std::path::Path::new(output_path).exists() {
            eprintln!("Output file already exists: {}", output_path);
            std::process::exit(1);
        }
    }



    // after de-duplication, the original order is still preserved
    let mut seen = HashSet::new();
    let targets: Vec<String> = args.target.into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect();

    let res = run_app(targets, args.count, args.interval, running.clone(), args.force_ipv6, args.multiple, args.view_type, args.output).await;

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
    output_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {

    // init terminal
    draw::init_terminal()?;

    // Create terminal instance
    let terminal = draw::init_terminal().unwrap();
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

    let mut point_num = 10;
    if view_type == "point" || view_type == "sparkline" {
        point_num = 200;
    }

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
            draw::draw_interface(
                &mut guard.terminal.as_mut().unwrap(),
                &view_type,
                &ip_data,
                &mut errs.lock().unwrap(),
            ).ok();
        }

        thread::spawn(move || {
            let mut output_file_handle = if let Some(ref output_path) = output_file {
                match std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(output_path)
                {
                    Ok(file) => Some(file),
                    Err(e) => {
                        let mut errs = errs.lock().unwrap();
                        errs.push(format!("Failed to create output file: {}", e));
                        None
                    }
                }
            } else {
                None
            };

            while let Ok(updated_data) = ping_update_rx.recv() {

                let mut ip_data = ip_data.lock().unwrap();

                let last_attr = updated_data.last_attr.clone();
                let addr = updated_data.addr.clone();
                let ip = updated_data.ip.clone();

                if let Some(pos) = ip_data.iter().position(|d| d.addr == updated_data.addr && d.ip == updated_data.ip) {
                    ip_data[pos] = updated_data;
                }


                if let Some(ref mut file) = output_file_handle {
                    use std::io::Write;

                    let latency_str = if last_attr == -1.0 {
                        "timeout".to_string()
                    } else {
                        format!("{:.2}ms", last_attr)
                    };

                    if let Err(e) = writeln!(file, "{} {} {}",
                                             addr,
                                             ip,
                                             latency_str
                    ) {
                        let mut errs = errs.lock().unwrap();
                        errs.push(format!("Failed to write to output file: {}", e));                    }
                }
                let mut guard = terminal_guard.lock().unwrap();
                draw::draw_interface(
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
                send_ping(addr, ip, errs.clone(), count, interval, running.clone(), ping_update_tx, point_num).await.unwrap();
            }
        });
        tasks.push(task)
    }

    for task in tasks {
        task.await?;
    }
    // restore terminal
    draw::restore_terminal(&mut terminal_guard.lock().unwrap().terminal.as_mut().unwrap())?;

    Ok(())
}