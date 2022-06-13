use std::{path::PathBuf, thread::JoinHandle};

use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};

pub fn start(mut buf: PathBuf, listen_on: u16) -> JoinHandle<Result<u8, libtor::Error>> {
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
}
