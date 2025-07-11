
use serde::{Serialize, de::DeserializeOwned};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::{formats::MessagePack, Framed as SerdeFramed};
use futures::prelude::*;

pub struct SlotClient<Transport, MsgTypeSend, MsgTypeRecv> {
    conn: SerdeFramed<Framed<Transport, LengthDelimitedCodec>,
                      MsgTypeRecv, MsgTypeSend, MessagePack<MsgTypeRecv, MsgTypeSend>>
}

impl<Transport, MsgTypeSend, MsgTypeRecv> SlotClient<Transport, MsgTypeSend, MsgTypeRecv>
where
    Transport: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    MsgTypeSend: Serialize + Unpin,
    MsgTypeRecv: DeserializeOwned + Unpin,
{
    /// Wrap the generic transport
    pub fn new(trans: Transport) -> Self {
        let trans_delimited = Framed::new(trans, LengthDelimitedCodec::new());
        let trans_serialized = SerdeFramed::new(trans_delimited, MessagePack::default());
        Self {
            conn: trans_serialized
        }
    }

    /// Manually send a slot protocol message to the server
    pub async fn send_msg(&mut self, msg: MsgTypeSend) -> Result<(), std::io::Error> {
        self.conn.send(msg).await
    }

    /// Loops forever 
    pub async fn heartbeat_task(mut self, msg: MsgTypeSend) -> std::io::Error {
        for msg in self.conn
        self.conn.split();
        self.conn.send(msg).await
    }
}