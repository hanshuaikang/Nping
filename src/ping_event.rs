#[derive(Debug, Clone)]
pub enum PingEvent {
    Success {
        addr: String,
        ip: String,
        rtt: f64,
    },
    Timeout {
        addr: String,
        ip: String,
    },
}