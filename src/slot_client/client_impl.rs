//! A provided implementation of a Slot module client

use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
    time::Duration,
};

const SOCK_FAIL_BEFORE_RESTART: u8 = 5;
// TODO: move these to protocol since they must be coordinated with the server
const SERVER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const SERVER_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
const SPAM_DELAY: Duration = Duration::from_secs(1);

/// Spawns a thread to handle communication with the slot server
///
/// # Errors
/// All error handling is encapsulated.
pub fn run_client(
    server_port: u16,
    my_name: crate::protocol::ValidName,
    my_http_port: u16,
) {
    let server_addr =
        SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), server_port);

    let _handle = std::thread::spawn(move || {
        log::info!("Starting Slot client");
        let mut fail_count = 0u8;
        // Restart loop
        loop {
            let my_slot_addr =
                SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), 0);
            // Create socket
            let socket = match UdpSocket::bind(my_slot_addr) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Unable to bind to localhost: \"{e}\"");

                    sleep(SPAM_DELAY);
                    continue;
                }
            };

            log::info!(
                "Slot client bound to {}",
                socket.local_addr().expect("Address is bound at this point")
            );

            let (len, my_name) = my_name.get();

            // Construct static messages
            let reg_msg = crate::protocol::SlotMsg {
                cmd: crate::protocol::MsgIds::Join as u8,
                module_http_port: my_http_port,
                name_len: len,
                name: my_name,
            }
            .as_bytes();

            let hb_msg = crate::protocol::SlotMsg {
                cmd: crate::protocol::MsgIds::Heartbeat as u8,
                module_http_port: 0,
                name_len: 0,
                name: [0; _],
            }
            .as_bytes();

            // Retry loop
            loop {
                // Check fail count
                if fail_count >= SOCK_FAIL_BEFORE_RESTART {
                    fail_count = 0;
                    log::warn!(
                        "Exceeded maximum fail count. Restarting Slot client"
                    );
                    break;
                }

                log::debug!("Sending join request to {server_addr}");

                // Request join
                if let Err(e) = socket.send_to(&reg_msg, server_addr) {
                    log::error!(
                        "Error sending join request on socket: \"{e}\""
                    );
                    fail_count += 1;

                    sleep(SPAM_DELAY);
                    continue;
                }

                let mut buf = [0u8; crate::protocol::PKT_LEN];

                socket
                    .set_read_timeout(Some(SERVER_RESPONSE_TIMEOUT))
                    .expect("The constant timeout is not zero");

                match socket.recv_from(&mut buf) {
                    Ok(_) => {
                        let msg = crate::protocol::SlotMsg::from_bytes(buf);

                        if msg.cmd == crate::protocol::MsgIds::ConfrimJoin as u8
                        {
                            log::info!(
                                "Received join confirmation from Slot server"
                            );
                        }
                    }
                    Err(e)
                        if [
                            std::io::ErrorKind::TimedOut,
                            std::io::ErrorKind::WouldBlock,
                        ]
                        .contains(&e.kind()) =>
                    {
                        log::debug!(
                            "No response from Slot server for join request"
                        );

                        sleep(SPAM_DELAY);
                        continue;
                    }
                    Err(e) => {
                        log::error!(
                            "Socket error while awaiting server response for \
                             join request: \"{e}\""
                        );
                        fail_count += 1;

                        sleep(SPAM_DELAY);
                        continue;
                    }
                }

                socket
                    .set_read_timeout(Some(SERVER_HEARTBEAT_TIMEOUT))
                    .expect("The constant timeout is not zero");

                // Heartbeat loop
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok(_) => {
                            log::debug!("Received heartbeat from Slot server");
                        }
                        Err(e)
                            if [
                                std::io::ErrorKind::TimedOut,
                                std::io::ErrorKind::WouldBlock,
                            ]
                            .contains(&e.kind()) =>
                        {
                            log::warn!(
                                "Slot server seems to be dead. No heartbeat \
                                 received"
                            );
                            break;
                        }
                        Err(e) => {
                            log::error!(
                                "Socket error while awaiting server heartbeat: \
                                 \"{e}\""
                            );
                            fail_count += 1;
                            break;
                        }
                    }

                    if let Err(e) = socket.send_to(&hb_msg, server_addr) {
                        log::error!(
                            "Error sending heartbeat reply on socket: \"{e}\""
                        );
                        fail_count += 1;
                        break;
                    }
                }

                sleep(SPAM_DELAY);
            }

            sleep(SPAM_DELAY);
        }
    });
}
