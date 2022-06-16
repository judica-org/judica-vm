use std::{any::Any, error::Error, path::PathBuf};

use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use tokio::task::JoinHandle;

pub fn start(
    mut buf: PathBuf,
    listen_on: u16,
) -> JoinHandle<Result<Result<u8, libtor::Error>, Box<dyn Any + Send>>> {
    tokio::task::spawn_blocking(move || {
        buf.push("onion");
        let mut tor = Tor::new();
        tor.flag(TorFlag::DataDirectory(buf.to_str().unwrap().into()));

        buf.push("chatserver");
        tor.flag(TorFlag::SocksPort(19050))
            .flag(TorFlag::HiddenServiceDir(buf.to_str().unwrap().into()))
            .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
            .flag(TorFlag::HiddenServicePort(
                TorAddress::Port(listen_on),
                None.into(),
            ))
            .start_background()
            .join()
    })
}
