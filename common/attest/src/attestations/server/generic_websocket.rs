use super::tungstenite_client_adaptor::ClientWebSocket;
use axum;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use futures::Future;
use futures::Sink;
use futures::Stream;
use std::pin::Pin;

type RecvOutput = Option<Result<Message, axum::Error>>;
pub trait WebSocketFunctionality
where
    Self: Stream<Item = Result<Message, axum::Error>>
        + Sink<Message, Error = axum::Error>
        + Send
        + 'static,
{
    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    fn t_recv<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = RecvOutput> + Send + 'a>>;

    /// Send a message.
    fn t_send<'a>(
        &'a mut self,
        msg: Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send + 'a>>;

    /// Gracefully close this WebSocket.
    fn t_close(self) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send>>;
}

impl WebSocketFunctionality for WebSocket {
    fn t_recv<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Option<Result<Message, axum::Error>>> + Send + 'a>> {
        Box::pin(self.recv())
    }

    fn t_send<'a>(
        &'a mut self,
        msg: Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send + 'a>> {
        Box::pin(self.send(msg))
    }

    fn t_close(self) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send>> {
        Box::pin(self.close())
    }
}

impl WebSocketFunctionality for ClientWebSocket {
    fn t_recv<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Option<Result<Message, axum::Error>>> + Send + 'a>> {
        Box::pin(self.recv())
    }

    fn t_send<'a>(
        &'a mut self,
        msg: Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send + 'a>> {
        Box::pin(self.send(msg))
    }

    fn t_close(self) -> Pin<Box<dyn Future<Output = Result<(), axum::Error>> + Send>> {
        Box::pin(self.close())
    }
}
