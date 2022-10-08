// Copyright (c) 2022 Judica Inc
// Copyright (c) 2019 Axum Contributors

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use axum::extract::ws::CloseFrame;
use axum::{extract::ws::Message, http::HeaderValue, Error};
use futures::SinkExt;
use futures_util::StreamExt;
use futures_util::{Sink, Stream};
use std::fmt::Debug;
use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_tungstenite::tungstenite as ts;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::Error as TungstenError;
use tokio_tungstenite::tungstenite::error::UrlError;
use tokio_tungstenite::tungstenite::handshake::client::Response as ClientResponse;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::globals::Globals;

use self::maybe_tor::MaybeTor;
mod maybe_tor {

    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio_socks::tcp::Socks5Stream;

    /// A stream that might be protected with TLS, or over tor
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum MaybeTor<S> {
        MaybeTls(S),
        TorHidden(Socks5Stream<S>),
    }

    impl<S> From<S> for MaybeTor<S> {
        fn from(v: S) -> Self {
            Self::MaybeTls(v)
        }
    }

    impl<S> From<Socks5Stream<S>> for MaybeTor<S> {
        fn from(v: Socks5Stream<S>) -> Self {
            Self::TorHidden(v)
        }
    }

    impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for MaybeTor<S> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            match self.get_mut() {
                MaybeTor::MaybeTls(ref mut s) => Pin::new(s).poll_read(cx, buf),
                MaybeTor::TorHidden(ref mut s) => Pin::new(s).poll_read(cx, buf),
            }
        }
    }

    impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for MaybeTor<S> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            match self.get_mut() {
                MaybeTor::MaybeTls(ref mut s) => Pin::new(s).poll_write(cx, buf),
                MaybeTor::TorHidden(ref mut s) => Pin::new(s).poll_write(cx, buf),
            }
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            match self.get_mut() {
                MaybeTor::MaybeTls(ref mut s) => Pin::new(s).poll_flush(cx),
                MaybeTor::TorHidden(ref mut s) => Pin::new(s).poll_flush(cx),
            }
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            match self.get_mut() {
                MaybeTor::MaybeTls(ref mut s) => Pin::new(s).poll_shutdown(cx),
                MaybeTor::TorHidden(ref mut s) => Pin::new(s).poll_shutdown(cx),
            }
        }
    }
}

#[derive(Debug)]
pub struct ClientWebSocket {
    inner: WebSocketStream<MaybeTlsStream<MaybeTor<TcpStream>>>,
    protocol: Option<HeaderValue>,
}

#[derive(Debug)]
pub enum TorWSError {
    TungstenError(TungstenError),
    SocksError(tokio_socks::Error),
}
impl std::fmt::Display for TorWSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
impl std::error::Error for TorWSError {}

impl From<TungstenError> for TorWSError {
    fn from(v: TungstenError) -> Self {
        Self::TungstenError(v)
    }
}

impl From<tokio_socks::Error> for TorWSError {
    fn from(v: tokio_socks::Error) -> Self {
        Self::SocksError(v)
    }
}
// TODO: Tor Support
impl ClientWebSocket {
    pub async fn connect(
        globals: &Arc<Globals>,
        url: String,
    ) -> Result<ClientWebSocket, TorWSError> {
        let (ws_stream, _) = Self::connect_async_with_config_tor(globals, url, None).await?;
        Ok(ClientWebSocket {
            inner: ws_stream,
            protocol: None,
        })
    }

    pub async fn connect_async_with_config_tor<R>(
        globals: &Arc<Globals>,
        request: R,
        config: Option<WebSocketConfig>,
    ) -> Result<
        (
            WebSocketStream<MaybeTlsStream<MaybeTor<TcpStream>>>,
            ClientResponse,
        ),
        TorWSError,
    >
    where
        R: IntoClientRequest + Unpin,
    {
        let request = request.into_client_request()?;

        let domain = request
            .uri()
            .host()
            .ok_or(TungstenError::Url(UrlError::NoHostName))?;
        let port = request
            .uri()
            .port_u16()
            .or_else(|| match request.uri().scheme_str() {
                Some("wss") => Some(443),
                Some("ws") => Some(80),
                _ => None,
            })
            .ok_or(TungstenError::Url(UrlError::UnsupportedUrlScheme))?;

        // TODO : resolve via tor
        let socket = if let Some(tor_port) = globals.config.tor.as_ref().map(|m| m.socks_port) {
            let proxy = format!("127.0.0.1:{}", tor_port);
            let addr = format!("{}:{}", domain, port);
            let socket = Socks5Stream::connect(proxy.as_str(), addr.as_str()).await?;
            socket.into()
        } else {
            let addr = format!("{}:{}", domain, port);
            let try_socket = TcpStream::connect(addr).await;
            let socket = try_socket.map_err(TungstenError::Io)?;
            // TODO: honor encryption?
            socket.into()
        };
        Ok(tokio_tungstenite::client_async_tls_with_config(request, socket, config, None).await?)
    }
}
impl ClientWebSocket {
    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    pub async fn recv(&mut self) -> Option<Result<Message, Error>> {
        let res = self.next().await;
        if res.is_none() {
            tracing::trace!("Attempted to read for Closed Client Socket")
        }
        if res.as_ref().and_then(|v| v.as_ref().ok()).is_none() {
            tracing::trace!("Err reading from Client Socket")
        }
        res
    }

    /// Send a message.
    pub async fn send(&mut self, msg: Message) -> Result<(), Error> {
        let res = SinkExt::send(self, msg).await;
        if res.is_err() {
            tracing::trace!("Attempted to send to a Closed Client Socket")
        }
        res
    }

    /// Gracefully close this WebSocket.
    pub async fn close(mut self) -> Result<(), Error> {
        self.inner.close(None).await.map_err(Error::new)
    }

    /// Return the selected WebSocket subprotocol, if one has been chosen.
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
}

pub fn into_tungstenite(m: Message) -> ts::Message {
    match m {
        Message::Text(text) => ts::Message::Text(text),
        Message::Binary(binary) => ts::Message::Binary(binary),
        Message::Ping(ping) => ts::Message::Ping(ping),
        Message::Pong(pong) => ts::Message::Pong(pong),
        Message::Close(Some(close)) => ts::Message::Close(Some(ts::protocol::CloseFrame {
            code: ts::protocol::frame::coding::CloseCode::from(close.code),
            reason: close.reason,
        })),
        Message::Close(None) => ts::Message::Close(None),
    }
}

fn from_tungstenite(message: ts::Message) -> Option<Message> {
    match message {
        ts::Message::Text(text) => Some(Message::Text(text)),
        ts::Message::Binary(binary) => Some(Message::Binary(binary)),
        ts::Message::Ping(ping) => Some(Message::Ping(ping)),
        ts::Message::Pong(pong) => Some(Message::Pong(pong)),
        ts::Message::Close(Some(close)) => Some(Message::Close(Some(CloseFrame {
            code: close.code.into(),
            reason: close.reason,
        }))),
        ts::Message::Close(None) => Some(Message::Close(None)),
        // we can ignore `Frame` frames as recommended by the tungstenite maintainers
        // https://github.com/snapview/tungstenite-rs/issues/268
        ts::Message::Frame(_) => None,
    }
}
impl Stream for ClientWebSocket {
    type Item = Result<Message, axum::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            match futures_util::ready!(self.inner.poll_next_unpin(cx)) {
                Some(Ok(msg)) => {
                    if let Some(msg) = from_tungstenite(msg) {
                        return Poll::Ready(Some(Ok(msg)));
                    }
                }
                Some(Err(err)) => return Poll::Ready(Some(Err(Error::new(err)))),
                None => return Poll::Ready(None),
            }
        }
    }
}

impl Sink<Message> for ClientWebSocket {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_ready(cx).map_err(Error::new)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.inner)
            .start_send(into_tungstenite(item))
            .map_err(Error::new)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx).map_err(Error::new)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx).map_err(Error::new)
    }
}
