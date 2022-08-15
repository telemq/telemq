use std::io;

use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use mqtt_packets::v_3_1_1::{ControlPacket, ControlPacketCodec};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::{Decoder, Encoder, Framed};
use warp::filters::ws::{Message, WebSocket};

pub enum NetConnection {
    Tcp(Framed<TcpStream, ControlPacketCodec>),
    Tls(Framed<TlsStream<TcpStream>, ControlPacketCodec>),
    Ws {
        websocket: WebSocket,
        codec: ControlPacketCodec,
        buf_in: BytesMut,
    },
}

impl NetConnection {
    pub fn new_tcp(framed_tcp: Framed<TcpStream, ControlPacketCodec>) -> Self {
        NetConnection::Tcp(framed_tcp)
    }

    pub fn new_tls(framed_tls: Framed<TlsStream<TcpStream>, ControlPacketCodec>) -> Self {
        NetConnection::Tls(framed_tls)
    }

    pub fn new_ws(arg: (WebSocket, ControlPacketCodec)) -> Self {
        NetConnection::Ws {
            websocket: arg.0,
            codec: arg.1,
            buf_in: BytesMut::new(),
        }
    }

    pub async fn next_packet(&mut self) -> Option<io::Result<ControlPacket>> {
        match self {
            NetConnection::Tcp(tcp_stream) => tcp_stream.next().await,
            NetConnection::Tls(tls_stream) => tls_stream.next().await,
            NetConnection::Ws {
                websocket,
                codec,
                buf_in: ref mut buf,
            } => loop {
                match websocket.next().await {
                    Some(Ok(message)) => {
                        buf.extend_from_slice(message.as_bytes());
                        let m = codec.decode(buf);
                        match m {
                            Ok(Some(packet)) => {
                                return Some(Ok(packet));
                            }
                            Ok(None) => {
                                continue;
                            }
                            Err(err) => {
                                return Some(Err(err));
                            }
                        }
                    }
                    Some(Err(err)) => {
                        return Some(Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("[Websocket Error] {:?}", err),
                        )));
                    }
                    None => {
                        return None;
                    }
                }
            },
        }
    }

    pub async fn send_packet(&mut self, control_packet: &ControlPacket) -> io::Result<()> {
        match self {
            NetConnection::Tcp(tcp_stream) => tcp_stream.send(&control_packet).await,
            NetConnection::Tls(tls_stream) => tls_stream.send(&control_packet).await,
            NetConnection::Ws {
                websocket, codec, ..
            } => {
                let mut bytes = BytesMut::new();
                match codec.encode(control_packet, &mut bytes) {
                    Ok(_) => websocket
                        .send(Message::binary(bytes.as_ref()))
                        .await
                        .map_err(|err| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                format!("[Websocket Error] {:?}", err),
                            )
                        }),
                    err => err,
                }
            }
        }
    }
}
