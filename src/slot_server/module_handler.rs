use slot_client::protocol::{self, ValidName};
use std::{
    net::{Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::{net::UdpSocket, time::sleep};

use crate::{cli::Args, store::ModuleStore};

// TODO: move these to protocol since they must be coordinated with the client
const PING_DELAY_SEC: Duration = Duration::from_secs(5);
const SOCK_FAIL_BEFORE_RESTART: u8 = 5;
const SPAM_DELAY: Duration = Duration::from_secs(1);
const DEATH_TIMER: Duration = Duration::from_secs(10);

pub async fn module_listener(module_store: ModuleStore, args: &Args) {
    let args = args.clone();

    tokio::spawn(async move {
        log::info!("Starting Slot module listener thread");
        let mut fail_count = 0u8;
        // Restart loop
        loop {
            let socket = match tokio::net::UdpSocket::bind(SocketAddr::new(
                std::net::Ipv4Addr::LOCALHOST.into(),
                args.slot_port,
            ))
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Unable to bind to socket address: \"{e}\"");
                    sleep(SPAM_DELAY).await;
                    continue;
                }
            };

            let mut buf = [0u8; protocol::PKT_LEN];

            // Listener loop
            loop {
                // await messages to join and handle them immediately
                // every once in a while, check for dead modules
                tokio::select! {
                    res = socket.recv_from(&mut buf) => {
                        match res {
                            Ok((_, from_addr)) => {
                                log::debug!("Slot listener received a packet");
                                let msg = protocol::SlotMsg::from_bytes(buf);

                                check_join_msg(
                                    &socket,
                                    &module_store,
                                    &from_addr,
                                    &msg,
                                    &mut fail_count
                                ).await;

                                check_ping_response(
                                    &module_store,
                                    &from_addr,
                                    &msg
                                ).await;
                            },
                            Err(e) => {
                                log::error!(
                                    "Error reading socket for incoming slot \
                                     requests: \"{e}\""
                                );

                                fail_count += 1;
                                sleep(SPAM_DELAY).await;
                                continue;
                            }
                        }
                    }
                    _ = tokio::time::sleep(PING_DELAY_SEC) => {
                        cleanup_dead(&socket, &module_store).await;

                        let failed = ping_all_modules(&socket, &module_store).await;
                        if failed {
                            fail_count += 1;
                        }

                        if fail_count > SOCK_FAIL_BEFORE_RESTART {
                            fail_count = 0;
                            log::warn!("Exceeded maximum fail count. \
                                        Restarting Slot listener thread");
                            break;
                        }
                    }
                };
            }
            sleep(SPAM_DELAY).await;
        }
    });
}

async fn check_join_msg(
    socket: &UdpSocket,
    module_store: &ModuleStore,
    from_addr: &SocketAddr,
    pkt: &protocol::SlotMsg,
    fail_count: &mut u8,
) {
    if pkt.cmd == protocol::MsgIds::Join as u8 {
        let resp = protocol::SlotMsg {
            cmd: protocol::MsgIds::ConfrimJoin as u8,
            module_http_port: 0,
            name_len: 0,
            name: [0; _],
        }
        .as_bytes();

        // copying out since compiler complains due to packed field access
        let http_port = pkt.module_http_port;

        let their_http_addr =
            SocketAddr::new(Ipv4Addr::LOCALHOST.into(), http_port);

        let name = ValidName::new(pkt.name_len, pkt.name);

        if let Err(e) = socket.send_to(&resp, from_addr).await {
            log::error!(
                "Error sending join acknowledgement on socket for module \
                 \"{name}\": \"{e}\". Module will not be added"
            );
            *fail_count += 1;
        }

        module_store
            .store_module(&name, &their_http_addr, from_addr)
            .await;

        log::info!("Added module \"{name}\". HTTP port: {http_port}");
    }
}

async fn check_ping_response(
    module_store: &ModuleStore,
    from_addr: &SocketAddr,
    pkt: &protocol::SlotMsg,
) {
    if pkt.cmd == protocol::MsgIds::Heartbeat as u8 {
        module_store.update_last_heard(from_addr).await;
    }
}

async fn cleanup_dead(socket: &UdpSocket, module_store: &ModuleStore) {
    // iterate over modules and check how long its been since we last heard them
    let mut dead = Vec::new();
    module_store.get_vec().await.retain(|module_info| {
        if Instant::now() - module_info.time_last_heard > DEATH_TIMER {
            dead.push(module_info.clone());

            false
        } else {
            true
        }
    });

    // remove the ones that have exceeded the limit
    let bye = protocol::SlotMsg {
        cmd: protocol::MsgIds::Bye as u8,
        module_http_port: 0,
        name_len: 0,
        name: [0; _],
    }
    .as_bytes();

    for module_info in dead {
        // this is just a courtesy in case somehow the module is alive
        // and messages are only going one way for some reason
        socket.send_to(&bye, module_info.slot_addr).await.ok(); // ignore error

        log::warn!(
            "Module \"{}\" has not responded for a while and will be \
            removed!",
            module_info.name
        );
    }
}

async fn ping_all_modules(
    socket: &UdpSocket,
    module_store: &ModuleStore,
) -> bool {
    let mut sock_fail = false;

    let ping_msg = protocol::SlotMsg {
        cmd: protocol::MsgIds::Heartbeat as u8,
        module_http_port: 0,
        name_len: 0,
        name: [0; _],
    }
    .as_bytes();

    for module_info in module_store.get_vec().await.iter_mut() {
        match socket.send_to(&ping_msg, module_info.slot_addr).await {
            Ok(_) => {
                // module_info.time_last_pinged = Instant::now();
                log::debug!(
                    "Pinged module: \"{}\" at {}",
                    module_info.name,
                    module_info.slot_addr
                )
            }
            Err(e) => {
                // error logged below. this is just for details
                log::debug!("Failed heartbeat send: \"{e}\"");
                sock_fail = true;
            }
        }
    }

    if sock_fail {
        log::error!("One or more attempts to send heartbeat to modules failed");
    }

    sock_fail
}
