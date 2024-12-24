use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    transport_channel, icmp_packet_iter,
    TransportChannelType::Layer4, TransportProtocol::Ipv4,
    TransportReceiver, TransportSender,
};
use std::net::{IpAddr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::Packet;
use crate::{ICMP_BUFFER_SIZE};
use crate::ip_data::IpData;

/// 初始化 ICMP 传输通道
pub fn init_transport_channel() -> Result<(TransportSender, TransportReceiver), Box<dyn std::error::Error>> {
    let (tx, rx) = transport_channel(1024, Layer4(Ipv4(IpNextHeaderProtocols::Icmp)))?;
    Ok((tx, rx))
}

/// 创建 ICMP 包迭代器
pub fn create_icmp_iter<'a>(rx: &'a mut TransportReceiver) -> pnet::transport::IcmpTransportChannelIterator<'a> {
    icmp_packet_iter(rx)
}

/// 解析目标地址
pub fn resolve_target(target: &str) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let addr = match IpAddr::from_str(target) {
        Ok(ip) => ip,
        Err(_) => {
            // 如果无法直接解析为 IP 地址，则尝试通过主机名解析
            match (target, 0).to_socket_addrs() {
                Ok(mut addrs) => match addrs.next() {
                    Some(socket_addr) => socket_addr.ip(),
                    None => {
                        eprintln!("cat not parse address: {}", target);
                        return Err("cat not parse address".into());
                    }
                },
                Err(_) => {
                    eprintln!("cat not parse address: {}", target);
                    return Err("cat not parse address".into());
                }
            }
        }
    };
    Ok(addr)
}


// rust
pub async fn send_ping<F>(
    addr: IpAddr,
    i: usize,
    count: usize,
    interval: u64,
    ip_data: Arc<Mutex<Vec<IpData>>>,
    mut callback: F,
    running: Arc<Mutex<bool>>,
    tx: Arc<Mutex<TransportSender>>,
    rx: Arc<Mutex<TransportReceiver>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() + Send + 'static,
{
    // 唯一 identifier
    let identifier = (std::process::id() as u16).wrapping_add(i as u16);
    // 给 seq 加偏移
    let mut seq = i as u16 * 1000 + 1;

    let mut last_sent_time = Instant::now();

    callback();

    while ip_data.lock().unwrap()[i].sent < count {
        // let (mut tx, mut rx) = network::init_transport_channel()?;
        let mut tx = tx.lock().unwrap();
        let mut rx = rx.lock().unwrap();
        let mut iter = create_icmp_iter(&mut *rx);

        if !*running.lock().unwrap() {
            break;
        }
        if last_sent_time.elapsed() < Duration::from_millis(interval) {
            continue;
        }

        let mut buffer = [0u8; ICMP_BUFFER_SIZE];
        let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
        packet.set_icmp_type(IcmpTypes::EchoRequest);
        packet.set_sequence_number(seq);
        packet.set_identifier(identifier);

        let checksum = pnet::packet::icmp::checksum(&IcmpPacket::new(packet.packet()).unwrap());
        packet.set_checksum(checksum);

        let now = Instant::now();
        tx.send_to(packet, addr)?;
        {
            let mut data = ip_data.lock().unwrap();
            data[i].sent += 1;
        }
        match iter.next_with_timeout(Duration::from_millis(interval))? {
            Some((reply, _)) if reply.get_icmp_type() == IcmpTypes::EchoReply => {
                if let Some(echo_reply) = EchoReplyPacket::new(reply.packet()) {
                    // 只匹配对应identifier和seq
                    if echo_reply.get_identifier() == identifier && echo_reply.get_sequence_number() == seq {
                        let rtt = now.elapsed().as_millis() as f64;
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
                }
            }
            Some(_) | None => {
                let mut data = ip_data.lock().unwrap();
                data[i].rtts.push_back(0.0);
                if data[i].rtts.len() > 10 {
                    data[i].rtts.pop_front();
                    data[i].pop_count += 1;
                }
            }
        }

        callback();
        seq = seq.wrapping_add(1);
        last_sent_time = Instant::now();
    }

    Ok(())
}