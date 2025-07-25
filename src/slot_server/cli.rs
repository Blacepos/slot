//! Defines the command line interface
//!
//! Adding attributes to this structure will add CLI options

use clap::Parser;
use std::net::IpAddr;

const DEFAULT_LOG_LEVEL: &str = "INFO";
const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_HTTP_PORT: &str = "8000";
const DEFAULT_HTTPS_PORT: &str = "8001";
const DEFAULT_SLOT_BIND: &str = "7568";

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Slot server")]
pub struct Args {
    /// Log level (ERROR, WARN, INFO, DEBUG, TRACE)
    #[arg(short='l', long="log", default_value=DEFAULT_LOG_LEVEL)]
    pub log_level: log::LevelFilter,

    /// The web server bind address e.g., "127.0.0.1"
    #[arg(short='w', long="web-interface", default_value=DEFAULT_BIND)]
    pub web_addr: IpAddr,

    /// The web server HTTP bind port e.g., "80"
    #[arg(short='H', long="http-bind", default_value=DEFAULT_HTTP_PORT)]
    pub http_port: u16,

    /// The web server HTTPS bind port e.g., "443"
    #[arg(short='S', long="https-bind", default_value=DEFAULT_HTTPS_PORT)]
    pub https_port: u16,

    /// The slot module listener bind port on localhost e.g., "7568"
    #[arg(short='s', long="slot-bind", default_value=DEFAULT_SLOT_BIND)]
    pub slot_port: u16,

    /// The PEM website certificate and public key for SSL
    #[arg(short = 'c', long = "cert")]
    pub cert_file: String,

    /// The PEM private key for SSL
    #[arg(short = 'k', long = "key")]
    pub key_file: String,

    /// The route that "/" redirects to. This allows the default route to
    /// redirect to a module route since the Slot server itself provides no
    /// content.
    #[arg(short = 'r', long = "default-redirect")]
    pub default_redirect: String,
}
