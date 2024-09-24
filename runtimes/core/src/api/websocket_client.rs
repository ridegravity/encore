use futures::sink::SinkExt;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::stream::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub struct WebSocketClient {
    send_channel: UnboundedSender<Message>,
    receive_channel: Mutex<UnboundedReceiver<Message>>,
    shutdown: watch::Sender<bool>,
}

impl WebSocketClient {
    pub async fn connect(request: http::Request<()>) -> WebSocketClient {
        let (connection, _resp) = tokio_tungstenite::connect_async(request).await.unwrap();

        let (ws_write, ws_read) = connection.split();

        let (send_channel_tx, send_channel_rx) = tokio::sync::mpsc::unbounded_channel();
        let (receive_channel_tx, receive_channel_rx) = tokio::sync::mpsc::unbounded_channel();

        let (shutdown, shutdown_watch) = watch::channel(false);

        tokio::spawn(send_to_ws(send_channel_rx, ws_write, shutdown_watch));
        tokio::spawn(ws_to_receive(ws_read, receive_channel_tx));

        WebSocketClient {
            send_channel: send_channel_tx,
            receive_channel: Mutex::new(receive_channel_rx),
            shutdown,
        }
    }

    pub fn send(&self, msg: bytes::Bytes) -> anyhow::Result<()> {
        let msg = String::from_utf8(msg.into())?;
        self.send_channel.send(Message::Text(msg))?;

        Ok(())
    }

    pub async fn recv(&self) -> Option<bytes::Bytes> {
        loop {
            let msg = self.receive_channel.lock().await.recv().await;

            match msg {
                Some(Message::Text(msg)) => return Some(msg.into()),
                Some(Message::Binary(vec)) => return Some(vec.into()),
                Some(msg) => {
                    log::trace!("unhandled message: {msg:?}");
                }
                None => return None,
            };
        }
    }

    pub fn close(&self) {
        if let Err(err) = self.shutdown.send(true) {
            log::debug!("error sending shutdown signal: {err}");
        }
    }
}

async fn send_to_ws(
    mut rx: UnboundedReceiver<Message>,
    mut ws: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                rx.close()
            },
            msg = rx.recv() => {
                if let Some(msg) = msg {
                    if let Err(err) = ws.send(msg).await {
                        log::debug!("failed sending over websocket: {err}");
                    }
                } else {
                    log::debug!("receive channel closed, shutting down");

                    if let Err(err) = ws.close().await {
                        log::debug!("closing websocket failed: {err}");
                    }

                    break;
                }
            }

        }
    }
}

async fn ws_to_receive(
    mut ws: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    tx: UnboundedSender<Message>,
) {
    loop {
        let msg = ws.next().await;

        match msg {
            Some(Ok(msg)) => {
                if let Err(err) = tx.send(msg) {
                    log::debug!("failed sending to receive channel: {err}");
                }
            }
            Some(Err(err)) => {
                log::debug!("received an error from websocket: {err}");
            }
            None => {
                log::debug!("websocket closed, shutting down");
                break;
            }
        }
    }
}
