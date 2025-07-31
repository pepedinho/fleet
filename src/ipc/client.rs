use std::{io::Write, os::unix::net::UnixStream};
use crate::{core::watcher::WatchRequest, ipc::server::DaemonRequest};



pub fn send_watch_request(req: DaemonRequest) -> Result<(), anyhow::Error> {
    let mut stream = UnixStream::connect("/tmp/fleetd.sock")?;

    let json = serde_json::to_string(&req)?;
    stream.write_all(json.as_bytes())?;
    Ok(())
}