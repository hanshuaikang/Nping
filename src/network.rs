use std::error::Error;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::{anyhow, Context};
use pinger::{ping, PingOptions, PingResult};
use crate::ip_data::IpData;

// get host ip address default to ipv4
pub(crate) fn get_host_ipaddr(host: &str, force_ipv6: bool) -> Result<String, Box<dyn Error>> {
    let ipaddr: Vec<_> = (host, 80)
        .to_socket_addrs()
        .with_context(|| format!("failed to resolve host: {}", host))?
        .map(|s| s.ip())
        .collect();

    if ipaddr.is_empty() {
        return Err(anyhow!("Could not resolve host: {}", host).into());
    }

    if force_ipv6 {
        let ipaddr = ipaddr
            .iter()
            .find(|ip| matches!(ip, IpAddr::V6(_)))
            .ok_or_else(|| anyhow!("Could not resolve '{}' to ipv6", host))?;
        return Ok(ipaddr.to_string());
    }

    let ipaddr = ipaddr
        .iter()
        .find(|ip| matches!(ip, IpAddr::V4(_)))
        .ok_or_else(|| anyhow!("Could not resolve '{}' to ipv4", host))?;


    Ok(ipaddr.to_string())
}


pub struct PingTask {
    addr: String,
    count: usize,
    interval: u64,
    index: usize,
    ip_data: Arc<Mutex<Vec<IpData>>>,
    running: Arc<Mutex<bool>>,
    errs: Arc<Mutex<Vec<String>>>,
}

impl PingTask {
    pub fn new(
        addr: String,
        count: usize,
        interval: u64,
        index: usize,
        ip_data: Arc<Mutex<Vec<IpData>>>,
        running: Arc<Mutex<bool>>,
        errs: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            addr,
            count,
            interval,
            index,
            ip_data,
            running,
            errs,
        }
    }

    pub async fn run<F>(&self, mut draw_ui: F) -> Result<(), Box<dyn Error>>
    where
        F: FnMut() + Send + 'static,
    {
        // interval defined 0.5s/every ping
        let interval = Duration::from_millis(self.interval);
        let options = PingOptions::new(
            self.addr.clone(),
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
                            let rtt_display: f64 =  format!("{:.2}", rtt).parse().unwrap();
                            update_stats(
                                self.ip_data.clone(),
                                self.index,
                                self.addr.parse().unwrap(),
                                rtt_display,
                            );
                        }
                        PingResult::Timeout(_) => {
                            update_timeout_stats(self.ip_data.clone(), self.index, self.addr.parse().unwrap());
                        }
                        PingResult::PingExited(status, err) => {
                            if status.code() != Option::from(0) {
                                let err = format!("host({}) ping err, reason: ping excited, status: {} err: {}", self.addr, err, status);
                                set_error(self.errs.clone(), err);
                            }
                        }
                        PingResult::Unknown(msg) => {
                            let err = format!("host({}) ping err, reason:unknown, err: {}", self.addr, msg);
                            set_error(self.errs.clone(), err);
                        }
                    }
                }
                Err(err) => {
                    let err = format!("host({}) ping err, reason: unknown, err: {}", self.addr, err);
                    set_error(self.errs.clone(), err);
                }
            }
            draw_ui();
        }

        Ok(())
    }
}

// send ping to the target address
pub async fn send_ping<F>(
    addr: String,
    i: usize,
    errs: Arc<Mutex<Vec<String>>>,
    count: usize,
    interval: i32,
    ip_data: Arc<Mutex<Vec<IpData>>>,
    mut draw_ui: F,
    running: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut() + Send + 'static,
{
    // draw ui first
    draw_ui();
    let task = PingTask::new(
        addr.to_string(),
        count,
        interval as u64,
        i,
        ip_data,
        running,
        errs,
    );
    Ok(task.run(draw_ui).await?)
}

// update statistics
fn update_stats(ip_data: Arc<Mutex<Vec<IpData>>>, i: usize, addr: IpAddr, rtt: f64) {
    let mut data = ip_data.lock().unwrap();
    data[i].ip = addr.to_string();
    data[i].received += 1;
    data[i].last_attr = rtt;
    data[i].rtts.push_back(rtt);
    if data[i].min_rtt == 0.0 || rtt < data[i].min_rtt {
        data[i].min_rtt = rtt;
    }
    if rtt > data[i].max_rtt {
        data[i].max_rtt = rtt;
    }
    if data[i].rtts.len() > 10 {
        data[i].rtts.pop_front();
        data[i].pop_count += 1;
    }
}

// update timeout statistics
fn update_timeout_stats(ip_data: Arc<Mutex<Vec<IpData>>>, i: usize, addr: IpAddr) {
    let mut data = ip_data.lock().unwrap();
    data[i].rtts.push_back(0.0);
    data[i].ip = addr.to_string();
    if data[i].rtts.len() > 10 {
        data[i].rtts.pop_front();
        data[i].timeout += 1;
        data[i].pop_count += 1;
    }
}

fn set_error(errs: Arc<Mutex<Vec<String>>>, err: String) {
    let mut err_list = errs.lock().unwrap();
    err_list.push(err)
}