//! Slot protocol definition

use std::{fmt::Display, str::FromStr};

pub enum MsgIds {
    // Client specific
    Join,

    // Server specific
    ConfrimJoin,
    RejectJoin,

    // Both client and server
    Heartbeat,
    Bye,
}

pub const MAX_MOD_NAME_LEN: usize = 20;
pub const PKT_LEN: usize = size_of::<SlotMsg>();

#[derive(Clone)]
#[repr(C, packed)]
pub struct SlotMsg {
    pub cmd: u8,
    pub module_http_port: u16,
    pub name_len: u8,
    pub name: [u8; MAX_MOD_NAME_LEN],
}

impl SlotMsg {
    pub fn as_bytes(&self) -> [u8; PKT_LEN] {
        let mut byte_swapped = self.clone();
        byte_swapped.module_http_port = byte_swapped.module_http_port.to_be();

        unsafe { *(&byte_swapped as *const Self as *const [u8; PKT_LEN]) }
    }

    pub fn from_bytes(bytes: [u8; PKT_LEN]) -> Self {
        let mut pkt = unsafe { (*(bytes.as_ptr() as *const Self)).clone() };

        pkt.module_http_port = u16::from_be(pkt.module_http_port);

        pkt
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidName(u8, [u8; MAX_MOD_NAME_LEN]);

impl FromStr for ValidName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ascii = s.as_ascii().ok_or("Invalid characters in string")?;
        let len = ascii.len();

        if len > MAX_MOD_NAME_LEN {
            return Err(format!(
                "String is too long. Must be at most {MAX_MOD_NAME_LEN}"
            ));
        }

        let valid_slice =
            if ascii.iter().all(|c| c.to_char().is_ascii_alphanumeric()) {
                Ok(s)
            } else {
                Err("Not all characters are alphanumeric")
            }?
            .as_bytes();

        let mut buf = [0u8; MAX_MOD_NAME_LEN];
        buf[..len].copy_from_slice(valid_slice);

        Ok(ValidName(len as u8, buf))
    }
}

impl Display for ValidName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(str::from_utf8(&self.1[..self.0 as usize]).unwrap(), f)
    }
}

impl ValidName {
    pub fn get(&self) -> (u8, [u8; MAX_MOD_NAME_LEN]) {
        (self.0, self.1)
    }

    pub fn new(length: u8, buf: [u8; MAX_MOD_NAME_LEN]) -> Self {
        Self(length, buf)
    }
}
