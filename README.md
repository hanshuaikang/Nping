<h1 align="center"> 🏎 Nping </h1>
<p align="center">
    <em>Nping is a Ping tool developed in Rust using the ICMP protocol. It supports concurrent Ping for multiple addresses, visual chart display, real-time data updates, and other features.</em>
</p>
<p align="center">
    <img src="docs/imgs/nb.gif" alt="Nping demo" width="30%">
</p>

[中文文档](./README_ZH.md)

**White**
<p align="center">
    <img src="docs/imgs/img.gif" alt="Nping demo" width="100%">
</p>

**Black**
<p align="center">
    <img src="docs/imgs/black.gif" alt="Nping demo" width="100%">
</p>


## Feature:
- Supports concurrent Ping for multiple addresses
- Supports visual latency display
- Real-time display of maximum, minimum, average latency, packet loss rate, and other metrics
- Support IpV4 and IpV6

## TODO:
- Support dynamic layout display
- Implement a better-looking UI

## Usage

```bash
nping www.baidu.com www.google.com www.apple.com www.sina.com -c 20 -i 2

🏎 Nping with concurrent,chart,multiple addresses,real-time data update

Usage: nping [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  target IP address or hostname to ping

Options:
  -c, --count <COUNT>        Number of pings to send [default: 65535]
  -i, --interval <INTERVAL>  Interval in seconds between pings [default: 0]
  -6, --force_ipv6           Force using IPv6
  -h, --help                 Print help
  -V, --version              Print version
```
