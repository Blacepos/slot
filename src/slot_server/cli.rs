//! Defines the command line interface
//!
//! Adding attributes to this structure will add CLI options

use clap::Parser;
use std::net::SocketAddr;

const DEFAULT_LOG_LEVEL: &str = "INFO";
const DEFAULT_WEB_BIND: &str = "127.0.0.1:8000";
const DEFAULT_SLOT_BIND: &str = "7568";

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Slot server")]
pub struct Args {
    /// Log level (ERROR, WARN, INFO, DEBUG, TRACE)
    #[arg(short='l', long="log", default_value=DEFAULT_LOG_LEVEL)]
    pub log_level: log::LevelFilter,

    /// The web server bind address e.g., "127.0.0.1:8000"
    #[arg(short='w', long="web-bind", default_value=DEFAULT_WEB_BIND)]
    pub web_addr: SocketAddr,

    /// The slot module listener bind port on localhost e.g., "7568"
    #[arg(short='s', long="slot-bind", default_value=DEFAULT_SLOT_BIND)]
    pub slot_port: u16,
}
