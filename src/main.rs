// 引入自定义模块
mod network;
mod ui;
mod terminal;
mod ip_data;
use clap::{App, Arg};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::ip_data::IpData;
use crate::network::send_ping;

const ICMP_BUFFER_SIZE: usize = 64;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let matches = App::new("rping")
        .version("1.0")
        .author("Your Name")
        .about("Ping with real-time plot")
        .arg(
            Arg::new("TARGET")
                .help("Target IP address or hostname to ping")
                .required(true)
                .index(1).
                multiple_values(true),
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .takes_value(true)
                .default_value("100")
                .help("Number of pings to send"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .takes_value(true)
                .default_value("0")
                .help("Interval in seconds between pings"),
        )
        .get_matches();

    let targets: Vec<&str> = matches.values_of("TARGET").unwrap().collect();
    let count: usize = matches.value_of("count").unwrap_or("100").parse().unwrap();
    let interval: u64 = matches.value_of("interval").unwrap_or("0").parse().unwrap();

    // 初始化终端界面
    ui::init_terminal()?;

    // 设置 Ctrl+C 处理
    let running = Arc::new(Mutex::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            let mut running = running.lock().unwrap();
            *running = false;
        })
            .expect("无法设置 Ctrl+C 处理器");
    }


    // 运行主应用程序
    let res = run_app(targets, count, interval, running.clone());


    // 处理可能的错误
    if let Err(err) = res {
        eprintln!("{}", err);
        std::process::exit(1);
    }
    Ok(())
}


// 应用程序主逻辑
fn run_app(
    targets: Vec<&str>,
    count: usize,
    interval: u64,
    running: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create terminal instance
    let terminal = ui::init_terminal().unwrap();
    let mut terminal_guard = terminal::TerminalGuard::new(terminal);

    // Initialize network transport channel
    let (mut tx, mut rx) = network::init_transport_channel()?;
    let mut iter = network::create_icmp_iter(&mut rx);

    // Define statistics variables
    let mut ip_data: Vec<IpData> = targets.iter().map(|&target| IpData {
        ip: String::from(""),
        addr:target.to_string(),
        rtts: VecDeque::new(),
        last_attr: 0.0,
        min_rtt: 0.0,
        max_rtt: 0.0,
        sent: 0,
        received: 0,
        pop_count: 0,
    }).collect();

    // Resolve target addresses
    let mut addrs = Vec::new();
    for target in &targets {
        let addr = network::resolve_target(target)?;
        addrs.push(addr);
    }

    let mut last_sent_time = Instant::now();
    let mut seq = 0u16;

    // Main loop
    while ip_data.iter().all(|data| data.sent < count) {
        if !*running.lock().unwrap() {
            break;
        }

        let interval = if interval == 0 { 500 } else { interval*1000 };

        // Check if a new ICMP request needs to be sent
        if last_sent_time.elapsed() >= Duration::from_millis(interval) {
            for (i, addr) in addrs.iter().enumerate() {
                last_sent_time = Instant::now();
                send_ping(&mut tx, &mut iter, addr, i, seq, interval, &mut ip_data)?;
            }

        }

        ui::draw_interface(&mut terminal_guard.terminal.as_mut().unwrap(), &ip_data)?;
        seq = seq.wrapping_add(1);
    }
    Ok(())
}