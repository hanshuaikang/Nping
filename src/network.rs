use std::collections::VecDeque;
use std::error::Error;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::SyncSender;
use std::time::Duration;
use anyhow::{anyhow, Context};

use pinger::{ping, PingOptions, PingResult};
use crate::ip_data::IpData;

// get host ip address default to ipv4
pub(crate) fn resolve_host_ips(host: &str, force_ipv6: bool) -> Result<Vec<IpAddr>, Box<dyn Error>> {

    // get ip address
    let ipaddr: Vec<_> = (host, 80)
        .to_socket_addrs()
        .with_context(|| format!("failed to resolve host: {}", host))?
        .map(|s| s.ip())
        .collect();

    if ipaddr.is_empty() {
        return Err(anyhow!("Could not resolve host: {}", host).into());
    }

    // filter ipv4 or ipv6
    let filtered_ips: Vec<IpAddr> = if force_ipv6 {
        ipaddr.into_iter()
            .filter(|ip| matches!(ip, IpAddr::V6(_)))
            .collect()
    } else {
        ipaddr.into_iter()
            .filter(|ip| matches!(ip, IpAddr::V4(_)))
            .collect()
    };

    if filtered_ips.is_empty() {
        return Err(anyhow!("Could not resolve host: {}", host).into());
    }

    Ok(filtered_ips)
}

pub(crate) fn get_host_ipaddr(host: &str, force_ipv6: bool) -> Result<String, Box<dyn Error>> {
    let ips = resolve_host_ips(host, force_ipv6)?;
    Ok(ips[0].to_string())
}

pub(crate) fn get_multiple_host_ipaddr(host: &str, force_ipv6: bool, multiple: usize) -> Result<Vec<String>, Box<dyn Error>> {
    let ips = resolve_host_ips(host, force_ipv6)?;
    Ok(ips.into_iter()
        .take(multiple)
        .map(|ip| ip.to_string())
        .collect())
}


pub struct PingTask {
    addr: String,
    ip: String,
    count: usize,
    interval: u64,
    running: Arc<Mutex<bool>>,
    errs: Arc<Mutex<Vec<String>>>,
}

impl PingTask {
    pub fn new(
        addr: String,
        ip: String,
        count: usize,
        interval: u64,
        running: Arc<Mutex<bool>>,
        errs: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            addr,
            ip,
            count,
            interval,
            running,
            errs,
        }
    }

    pub async fn run(&self, ping_update_tx: Arc<SyncSender<IpData>>) -> Result<(), Box<dyn Error>>
    {
        let mut ip_data = IpData {
            addr: self.addr.clone(),
            ip: self.ip.clone(),
            rtts: VecDeque::new(),
            last_attr: 0.0,
            min_rtt: 0.0,
            max_rtt: 0.0,
            timeout: 0,
            received: 0,
            pop_count: 0,
        };
        // interval defined 0.5s/every ping
        let interval = Duration::from_millis(self.interval);
        let options = PingOptions::new(
            self.ip.clone(),
            interval,
            None,
        );

        // star ping
        let stream = ping(options)?;

        for _ in 0..self.count {
            // if ctrl+c is pressed, break the loop
            if !*self.running.lock().unwrap() {
                break;
            }
            match stream.recv() {
                Ok(result) => {
                    match result {
                        PingResult::Pong(duration, _size) => {
                            // calculate rtt
                            let rtt = duration.as_secs_f64() * 1000.0;
                            let rtt_display: f64 = format!("{:.2}", rtt).parse().unwrap();
                            update_stats(
                                &mut ip_data,
                                self.ip.parse().unwrap(),
                                rtt_display,
                            );
                        }
                        PingResult::Timeout(_) => {
                            update_timeout_stats(&mut ip_data, self.ip.parse().unwrap());
                        }
                        PingResult::PingExited(status, err) => {
                            if status.code() != Option::from(0) {
                                let err = format!("host({}) ping err, reason: ping excited, status: {} err: {}", self.ip, err, status);
                                set_error(self.errs.clone(), err);
                            }
                        }
                        PingResult::Unknown(msg) => {
                            let err = format!("host({}) ping err, reason:unknown, err: {}", self.ip, msg);
                            set_error(self.errs.clone(), err);
                        }
                    }
                }
                Err(err) => {
                    let err = format!("host({}) ping err, reason: unknown, err: {}", self.ip, err);
                    set_error(self.errs.clone(), err);
                }
            }


            // send ping data to update
            let _ = ping_update_tx.send(ip_data.clone());
        }

        Ok(())
    }
}

// send ping to the target address
pub async fn send_ping(
    addr: String,
    ip: String,
    errs: Arc<Mutex<Vec<String>>>,
    count: usize,
    interval: i32,
    running: Arc<Mutex<bool>>,
    ping_update_tx: Arc<SyncSender<IpData>>,
) -> Result<(), Box<dyn Error>>
{
    // draw ui first
    let task = PingTask::new(
        addr.to_string(),
        ip,
        count,
        interval as u64,
        running,
        errs,
    );
    Ok(task.run(ping_update_tx).await?)
}

// update statistics
fn update_stats(ip_data: &mut IpData, ip: IpAddr, rtt: f64) {
    ip_data.ip = ip.to_string();
    ip_data.received += 1;
    ip_data.last_attr = rtt;
    ip_data.rtts.push_back(rtt);
    if ip_data.min_rtt == 0.0 || rtt < ip_data.min_rtt {
        ip_data.min_rtt = rtt;
    }
    if rtt > ip_data.max_rtt {
        ip_data.max_rtt = rtt;
    }
    if ip_data.rtts.len() > 10 {
        ip_data.rtts.pop_front();
        ip_data.pop_count += 1;
    }
}

// update timeout statistics
fn update_timeout_stats(ip_data: &mut IpData, ip: IpAddr) {
    ip_data.rtts.push_back(-1.0);
    ip_data.last_attr = -1.0;
    ip_data.ip = ip.to_string();
    ip_data.timeout += 1;
    if ip_data.rtts.len() > 10 {
        ip_data.rtts.pop_front();
        ip_data.pop_count += 1;
    }
}

fn set_error(errs: Arc<Mutex<Vec<String>>>, err: String) {
    let mut err_list = errs.lock().unwrap();
    err_list.push(err)
}