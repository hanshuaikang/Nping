use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    transport_channel, icmp_packet_iter,
    TransportChannelType::Layer4, TransportProtocol::Ipv4,
    TransportReceiver, TransportSender,
};
use std::net::{IpAddr, ToSocketAddrs};
use std::str::FromStr;
use std::time::{Duration, Instant};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::Packet;
use crate::ICMP_BUFFER_SIZE;
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


pub fn send_ping(
    tx: &mut TransportSender,
    iter: &mut pnet::transport::IcmpTransportChannelIterator,
    addr: &IpAddr,
    i: usize,
    seq: u16,
    interval: u64,
    ip_data: &mut Vec<IpData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; ICMP_BUFFER_SIZE];
    let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_sequence_number(seq);
    packet.set_identifier(0);
    let checksum = pnet::packet::icmp::checksum(&IcmpPacket::new(packet.packet()).unwrap());
    packet.set_checksum(checksum);

    let now = Instant::now();
    tx.send_to(packet, *addr)?;
    ip_data[i].sent += 1; // sent

    // Wait for reply
    let timeout = Duration::from_secs(interval);
    match iter.next_with_timeout(timeout)? {
        Some((reply, _)) => {
            if reply.get_icmp_type() == IcmpTypes::EchoReply {
                if let Some(echo_reply) = EchoReplyPacket::new(reply.packet()) {
                    if echo_reply.get_sequence_number() == seq {
                        let rtt = now.elapsed().as_millis() as f64;
                        ip_data[i].ip = addr.to_string(); // ip
                        ip_data[i].received += 1; // received
                        ip_data[i].last_attr = rtt; // last_attr
                        ip_data[i].rtts.push_back(rtt); // rtts
                        if ip_data[i].min_rtt == 0.0 || rtt < ip_data[i].min_rtt {
                            ip_data[i].min_rtt = rtt; // min_rtt
                        }
                        if rtt > ip_data[i].max_rtt {
                            ip_data[i].max_rtt = rtt; // max_rtt
                        }
                        if ip_data[i].rtts.len() > 10 {
                            ip_data[i].rtts.pop_front();
                            ip_data[i].pop_count += 1; // pop_count
                        }
                    }
                }
            }
        }
        None => {
            ip_data[i].rtts.push_back(0.0); // timeout
            if ip_data[i].rtts.len() > 10 {
                ip_data[i].rtts.pop_front();
                ip_data[i].pop_count += 1; // pop_count
            }
        }
    }
    Ok(())
}