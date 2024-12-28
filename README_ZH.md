
<h1 align="center"> 🏎 Nping </h1>
<p align="center">
    <em>Nping 是一个基于 Rust 开发使用 ICMP 协议的 Ping 工具, 支持多地址并发 Ping, 可视化图表展示, 数据实时更新等特性 </em>
</p>


![img.png](docs/imgs/img.png)


## Feature:
- 支持多地址并发同时 Ping
- 支持可视化延迟展示
- 实时最大最小平均延迟丢包率等指标展示

## TODO:
- 支持动态布局展示
- 更好看的 UI 实现

## Usage

```bash

# 由于使用 pnet 包实现, 需要 root 权限
sudo sudo nping www.baidu.com www.google.com www.apple.com www.sina.com -c 20

# nping --help

nping with concurrent, chart, multiple addresses, real -time data update

Usage: nping [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  target IP address or hostname to ping

Options:
  -c, --count <COUNT>        Number of pings to send [default: 10000]
  -i, --interval <INTERVAL>  Interval in seconds between pings [default: 0]
  -s, --size <SIZE>          packet size [default: 32]
  -h, --help                 Print help
  -V, --version              Print version

```

