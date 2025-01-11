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

**Table View**
<p align="center">
    <img src="docs/imgs/table.gif" alt="Nping demo" width="100%">
</p>

## Installation

#### MacOS Homebrew
```bash
brew tap hanshuaikang/nping
brew install nping

nping --help
```

## Feature:
- Supports concurrent Ping for multiple addresses
- Supports visual latency display
- Real-time display of maximum, minimum, average latency, packet loss rate, and other metrics
- Support IpV4 and IpV6
- Supports concurrent pinging of n ip's under one address.

## Roadmap:

- Show country and city of IP
- Add host sub-command, support to show the details of ip address corresponding to the domain name.
- Optimize UI interface, add more dynamic effects.
- When there are multiple addresses, the display will be sorted according to the average delay at the end.

## Usage

```bash
nping www.baidu.com www.google.com www.apple.com www.sina.com -c 20 -i 2

nping --help

🏎 Nping mean NB Ping, A Ping Tool in Rust with Real-Time Data and Visualizations

Usage: nping [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  target IP address or hostname to ping

Options:
  -c, --count <COUNT>        Number of pings to send [default: 65535]
  -i, --interval <INTERVAL>  Interval in seconds between pings [default: 0]
  -6, --force_ipv6           Force using IPv6
  -m, --multiple <MULTIPLE>  Specify the maximum number of target addresses, Only works on one target address [default: 0]
  -h, --help                 Print help
  -V, --version              Print version
```

## Acknowledgements
Thanks to these people for their feedback and suggestions for 🏎Nping!

| [zx4i](https://github.com/zx4i) | [snail2sky](https://github.com/snail2sky) | [shenshouer](https://github.com/shenshouer) | [vnt-dev](https://github.com/vnt-dev) | [qingyuan0o0](https://github.com/qingyuan0o0) 
| [Onlywzr](https://github.com/Onlywzr)


## Star History
[![Star History Chart](https://api.star-history.com/svg?repos=hanshuaikang/Nping&type=Date)](https://star-history.com/#hanshuaikang/Nping&Date)