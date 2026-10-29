//! The small service every q2 device runs so that its peers can ask it things
//! that RustDesk's protocol cannot carry: the physical size of its monitors
//! (R-LAYOUT-2, R-LAYOUT-3), a claim of control (R-DEV-2), and later the layout
//! documents (R-LAYOUT-7).
//!
//! One request per connection: the client sends one line of JSON, the server
//! answers with one line of JSON and closes. The server only answers addresses
//! that `allow` accepts (paired Tailscale peers), and drops everyone else
//! without a word.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use q2_layout::OsPanel;
use serde::{Deserialize, Serialize};

/// RustDesk uses 21115 to 21119; this is the next free one.
pub const PORT: u16 = 21130;
/// Claims of control go to the process that owns the input hooks, which is the
/// main window. It listens here, so the `--server` process holding `PORT` does
/// not swallow them.
pub const CLAIM_PORT: u16 = 21131;

const MAX_REQUEST_BYTES: u64 = 4096;
const MAX_REPLY_BYTES: u64 = 256 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_CONNECTIONS: usize = 8;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    Monitors,
    /// "The person is using my own mouse or keyboard." Whoever is driving me
    /// should let go.
    Claim,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Monitors {
    panels: Vec<OsPanel>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Claimed {
    /// True when the receiver was driving the sender and has let go.
    released: bool,
}

pub type Allow = Arc<dyn Fn(IpAddr) -> bool + Send + Sync>;
pub type Provider = Arc<dyn Fn() -> Vec<OsPanel> + Send + Sync>;
/// Called with the address of a device that claims control. Returns whether
/// this device was driving it and has let go.
pub type OnClaim = Arc<dyn Fn(IpAddr) -> bool + Send + Sync>;

pub struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl Server {
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop accepting connections.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(200));
    }
}

/// Start answering on `bind` in background threads.
pub fn serve(
    bind: SocketAddr,
    allow: Allow,
    provider: Provider,
    on_claim: OnClaim,
) -> io::Result<Server> {
    let listener = TcpListener::bind(bind)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicUsize::new(0));
    let flag = stop.clone();
    thread::Builder::new()
        .name("q2-sync-accept".to_owned())
        .spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else {
                    continue;
                };
                if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                    active.fetch_sub(1, Ordering::SeqCst);
                    continue;
                }
                let allow = allow.clone();
                let provider = provider.clone();
                let on_claim = on_claim.clone();
                let counter = active.clone();
                let spawned = thread::Builder::new()
                    .name("q2-sync-conn".to_owned())
                    .spawn(move || {
                        let _ = answer(stream, &allow, &provider, &on_claim);
                        counter.fetch_sub(1, Ordering::SeqCst);
                    });
                if spawned.is_err() {
                    active.fetch_sub(1, Ordering::SeqCst);
                }
            }
        })?;
    Ok(Server { addr, stop })
}

fn answer(
    mut stream: TcpStream,
    allow: &Allow,
    provider: &Provider,
    on_claim: &OnClaim,
) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let peer = stream.peer_addr()?.ip();
    if !allow(peer) {
        return Ok(());
    }
    let mut line = String::new();
    BufReader::new((&stream).take(MAX_REQUEST_BYTES)).read_line(&mut line)?;
    let Ok(request) = serde_json::from_str::<Request>(line.trim()) else {
        return Ok(());
    };
    let reply = match request {
        Request::Monitors => serde_json::to_vec(&Monitors { panels: provider() }),
        Request::Claim => serde_json::to_vec(&Claimed {
            released: on_claim(peer),
        }),
    }
    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    stream.write_all(&reply)?;
    stream.write_all(b"\n")?;
    stream.shutdown(Shutdown::Both)
}

/// Send one request and read the one-line answer.
fn ask(addr: SocketAddr, timeout: Duration, request: &Request) -> io::Result<String> {
    let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let request = serde_json::to_vec(request)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    stream.write_all(&request)?;
    stream.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new((&stream).take(MAX_REPLY_BYTES)).read_line(&mut line)?;
    if line.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the device did not answer",
        ));
    }
    Ok(line)
}

/// Ask the device at `addr` for the monitors it has.
pub fn fetch_monitors(addr: SocketAddr, timeout: Duration) -> io::Result<Vec<OsPanel>> {
    let line = ask(addr, timeout, &Request::Monitors)?;
    let reply: Monitors = serde_json::from_str(line.trim())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(reply.panels)
}

/// Tell the device at `addr` that the person is now using this device. Returns
/// whether that device was driving this one and has let go.
pub fn send_claim(addr: SocketAddr, timeout: Duration) -> io::Result<bool> {
    let line = ask(addr, timeout, &Request::Claim)?;
    let reply: Claimed = serde_json::from_str(line.trim())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(reply.released)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(index: u32, w: u32, h: u32, wmm: f64, hmm: f64) -> OsPanel {
        OsPanel {
            index,
            x: 0,
            y: 0,
            width: w,
            height: h,
            width_mm: wmm,
            height_mm: hmm,
            scale: 1.0,
        }
    }

    fn start(allow: bool, panels: Vec<OsPanel>) -> Server {
        start_claiming(allow, panels, false)
    }

    fn start_claiming(allow: bool, panels: Vec<OsPanel>, releases: bool) -> Server {
        let allow: Allow = Arc::new(move |_| allow);
        let provider: Provider = Arc::new(move || panels.clone());
        let on_claim: OnClaim = Arc::new(move |ip| releases && ip.is_loopback());
        serve("127.0.0.1:0".parse().unwrap(), allow, provider, on_claim).unwrap()
    }

    #[test]
    fn a_paired_peer_gets_the_monitors() {
        let panels = vec![
            panel(0, 2560, 1440, 597.0, 336.0),
            panel(1, 1920, 1080, 521.0, 293.0),
        ];
        let server = start(true, panels.clone());
        let got = fetch_monitors(server.local_addr(), Duration::from_secs(2)).unwrap();
        assert_eq!(got, panels);
        server.stop();
    }

    #[test]
    fn an_unknown_address_gets_silence() {
        let server = start(false, vec![panel(0, 1920, 1080, 521.0, 293.0)]);
        // Depending on timing the client sees a closed or a reset connection.
        assert!(fetch_monitors(server.local_addr(), Duration::from_secs(2)).is_err());
        server.stop();
    }

    #[test]
    fn nonsense_and_oversized_requests_get_silence() {
        let server = start(true, vec![panel(0, 1920, 1080, 521.0, 293.0)]);
        for payload in [b"hello\n".to_vec(), vec![b'a'; 10_000]] {
            let mut stream = TcpStream::connect(server.local_addr()).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let _ = stream.write_all(&payload);
            let mut reply = Vec::new();
            let _ = stream.read_to_end(&mut reply);
            assert!(reply.is_empty());
        }
        // The server is still fine afterwards.
        assert!(fetch_monitors(server.local_addr(), Duration::from_secs(2)).is_ok());
        server.stop();
    }

    #[test]
    fn nothing_listening_is_an_error_not_a_hang() {
        let server = start(true, vec![]);
        let addr = server.local_addr();
        server.stop();
        thread::sleep(Duration::from_millis(100));
        assert!(fetch_monitors(addr, Duration::from_millis(500)).is_err());
    }

    #[test]
    fn a_claim_reports_whether_the_receiver_let_go() {
        let server = start_claiming(true, vec![], true);
        assert!(send_claim(server.local_addr(), Duration::from_secs(2)).unwrap());
        server.stop();
        let server = start_claiming(true, vec![], false);
        assert!(!send_claim(server.local_addr(), Duration::from_secs(2)).unwrap());
        server.stop();
    }

    #[test]
    fn a_claim_from_an_unknown_address_is_ignored() {
        let server = start_claiming(false, vec![], true);
        assert!(send_claim(server.local_addr(), Duration::from_secs(2)).is_err());
        server.stop();
    }
}
