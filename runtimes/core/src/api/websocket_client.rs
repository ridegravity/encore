use futures::sink::SinkExt;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::stream::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub struct WebSocketClient {
    send_channel: UnboundedSender<Message>,
    receive_channel: Mutex<UnboundedReceiver<Message>>,
}

impl WebSocketClient {
    pub async fn connect(request: http::Request<()>) -> WebSocketClient {
        let (connection, _resp) = tokio_tungstenite::connect_async(request).await.unwrap();

        let (write, read) = connection.split();

        let (send_channel_tx, send_channel_rx) = tokio::sync::mpsc::unbounded_channel();
        let (receive_channel_tx, receive_channel_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(send_to_ws(send_channel_rx, write));
        tokio::spawn(ws_to_receive(read, receive_channel_tx));

        WebSocketClient {
            send_channel: send_channel_tx,
            receive_channel: Mutex::new(receive_channel_rx),
        }
    }

    pub fn send(&self, msg: String) {
        if let Err(err) = self.send_channel.send(Message::Text(msg)) {
            log::error!("failed sending to send_channel: {err}");
        }
    }

    pub async fn recv(&self) -> Option<String> {
        loop {
            let msg = self.receive_channel.lock().await.recv().await;

            match msg {
                Some(Message::Text(msg)) => return Some(msg),
                Some(msg) => {
                    log::warn!("unhandled message: {msg:?}");
                }
                None => return None,
            };
        }
    }

    pub fn close(&self) {
        // TODO implement close
    }
}

async fn send_to_ws(
    mut rx: UnboundedReceiver<Message>,
    mut ws: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) {
    loop {
        let msg = rx.recv().await;

        if let Some(msg) = msg {
            if let Err(err) = ws.send(msg).await {
                log::error!("failed sending over websocket: {err}");
            }
        } else {
            log::warn!("receive channel closed, shutting down");
            break;
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
                    log::error!("failed sending to receive channel: {err}");
                }
            }
            Some(Err(err)) => {
                log::warn!("received an error from websocket: {err}");
            }
            None => {
                log::warn!("websocket closed, shutting down");
                break;
            }
        }
    }
}
