use serde::{Deserialize, Serialize};


pub const MODULE_NAME_LEN_MAX: usize = 32;
pub const PACKET_SIZE_MAX: usize = 64;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMsg {
    Join {
        name_len: u8,
        name: String, // Use a different type which disallows non-url-safe chars
        http_port: u16,
    },
    ReplyHeartbeat,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMsg {
    ConfrimJoin,
    Heartbeat,
}
