//! WestPlay Lite's first step of casting from a phone: the truck screen shows up in the phone's cast list, and what
//! the phone asks of it is reported (`Event`). Nothing is played yet: the pairing and the playing come once a real
//! phone's requests have been seen (they're in WestPlay's log, `[cast]`).
//!
//! How a phone finds a "TV" is DIAL, on the local network:
//!
//! 1. SSDP: the phone multicasts an `M-SEARCH` to 239.255.255.250:1900; a DIAL device answers it, by unicast, with
//!    the address of its description (`LOCATION`).
//! 2. The description (`/dd.xml`) names the device, and its `Application-URL` header says where its apps are.
//! 3. A phone's YouTube asks `GET /apps/YouTube` (is it there, running?) and, once the player taps the device,
//!    `POST /apps/YouTube` with a body (`pairingCode=...&v=...`): the app is to start, and the YouTube on the TV
//!    pairs with the phone by that code (YouTube's own protocol, not here yet).
//!
//! Only the standard library and `socket2` (SSDP's port is shared with Windows' own SSDP service, which takes
//! `SO_REUSEADDR` before binding).

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, Type};

const SSDP_GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
/// The first port tried for the description and the apps; any free one when it's taken.
const HTTP_PORT: u16 = 56790;
/// What a DIAL device answers a search for.
const DIAL_TARGET: &str = "urn:dial-multiscreen-org:service:dial:1";
/// The one app the truck screen offers.
const APP: &str = "YouTube";
/// The biggest request (head and body) read: a phone's launch is a few hundred bytes.
const MAX_REQUEST: usize = 16 * 1024;

/// What happened, for WestPlay's log (and, later, for the player).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Listening: the description is on this port.
    Started { http_port: u16 },
    /// A phone searched for devices (any search is told, one it answered or not).
    Search { from: SocketAddr, target: String, answered: bool },
    /// Any request on the DIAL server, as the phone sent it (the body cut at 512 bytes).
    Request { from: SocketAddr, method: String, path: String, body: String },
    /// The player tapped the device: the app is to start, with what the phone sent.
    Launch { app: String, body: String },
    /// The phone stopped the app.
    Stop { app: String },
}

type Report = Arc<dyn Fn(Event) + Send + Sync>;

/// The device as the phone sees it.
struct Device {
    name: String,
    uuid: String,
    /// Whether the phone started the app (and hasn't stopped it).
    running: Mutex<bool>,
}

/// Casting is on while this lives: dropping it stops listening.
pub struct Handle {
    stop: Arc<AtomicBool>,
    threads: Mutex<Vec<JoinHandle<()>>>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut threads) = self.threads.lock() {
            for thread in threads.drain(..) {
                let _ = thread.join();
            }
        }
    }
}

/// Starts being a cast target called `name`. Errors when the network can't be listened on (reported as such: casting
/// is then off).
pub fn start(name: &str, report: impl Fn(Event) + Send + Sync + 'static) -> io::Result<Handle> {
    let report: Report = Arc::new(report);
    let stop = Arc::new(AtomicBool::new(false));
    let device = Arc::new(Device { name: name.to_string(), uuid: uuid_for(name), running: Mutex::new(false) });

    let ssdp = bind_ssdp()?;
    let listener = bind_http()?;
    let port = listener.local_addr()?.port();

    let mut threads = Vec::new();
    {
        let (stop, device, report) = (stop.clone(), device.clone(), report.clone());
        threads.push(thread::Builder::new().name("westplay-cast-ssdp".into()).spawn(move || {
            ssdp_loop(&ssdp, port, &device, &report, &stop);
        })?);
    }
    {
        let (stop, report) = (stop.clone(), report.clone());
        threads.push(thread::Builder::new().name("westplay-cast-http".into()).spawn(move || {
            http_loop(&listener, &device, &report, &stop);
        })?);
    }
    report(Event::Started { http_port: port });
    Ok(Handle { stop, threads: Mutex::new(threads) })
}

// ---------------------------------------------------------------------------------------------------- SSDP

fn bind_ssdp() -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    // Windows' own SSDP service listens on this port too: shared, or ours is refused.
    socket.set_reuse_address(true)?;
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, SSDP_PORT)).into())?;
    // The default network, and the one the route to the outside takes (they may differ: VPNs, virtual adapters).
    let _ = socket.join_multicast_v4(&SSDP_GROUP, &Ipv4Addr::UNSPECIFIED);
    if let Some(ip) = primary_ip() {
        let _ = socket.join_multicast_v4(&SSDP_GROUP, &ip);
    }
    socket.set_read_timeout(Some(Duration::from_millis(500)))?;
    Ok(socket.into())
}

/// This computer's address on the network that leads out (no packet is sent: a UDP socket "connected" only picks
/// the route).
fn primary_ip() -> Option<Ipv4Addr> {
    local_ip_towards("192.0.2.1:9".parse().ok()?)
}

/// This computer's address on the network `peer` is reached by.
fn local_ip_towards(peer: SocketAddr) -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(peer).ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

fn ssdp_loop(socket: &UdpSocket, http_port: u16, device: &Device, report: &Report, stop: &AtomicBool) {
    let mut buffer = [0u8; 2048];
    while !stop.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buffer) {
            Ok((length, from)) => {
                let text = String::from_utf8_lossy(&buffer[..length]);
                let Some(target) = search_target(&text) else { continue };
                let answer = answers(&target).then(|| {
                    // The address the phone can reach: this computer's on the network it asked from.
                    let ip = local_ip_towards(from)?;
                    Some(search_reply(device, ip, http_port))
                });
                let answered = match answer.flatten() {
                    Some(reply) => socket.send_to(reply.as_bytes(), from).is_ok(),
                    None => false,
                };
                report(Event::Search { from, target, answered });
            }
            // The timeout (so `stop` is looked at), or a reply that bounced (Windows reports it on the next read).
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
}

/// The `ST` of an `M-SEARCH`, or `None` for anything else (announcements, other devices' answers).
fn search_target(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if !lines.next()?.trim_start().to_ascii_uppercase().starts_with("M-SEARCH") {
        return None;
    }
    lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim().eq_ignore_ascii_case("st").then(|| value.trim().to_string())
    })
}

/// Whether a search for `target` is one a DIAL device answers.
fn answers(target: &str) -> bool {
    target == DIAL_TARGET || target.eq_ignore_ascii_case("ssdp:all") || target == "upnp:rootdevice"
}

fn search_reply(device: &Device, ip: Ipv4Addr, http_port: u16) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nEXT:\r\nLOCATION: http://{ip}:{http_port}/dd.xml\r\n\
         SERVER: Windows/10 UPnP/1.1 WestPlay/1.0\r\nST: {DIAL_TARGET}\r\nUSN: uuid:{}::{DIAL_TARGET}\r\n\
         BOOTID.UPNP.ORG: 1\r\n\r\n",
        device.uuid
    )
}

// ---------------------------------------------------------------------------------------------------- DIAL

fn bind_http() -> io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::UNSPECIFIED, HTTP_PORT)).or_else(|_| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)))
}

fn http_loop(listener: &TcpListener, device: &Arc<Device>, report: &Report, stop: &AtomicBool) {
    if listener.set_nonblocking(true).is_err() {
        return;
    }
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, from)) => {
                let (device, report) = (device.clone(), report.clone());
                let _ = thread::Builder::new().name("westplay-cast-request".into()).spawn(move || {
                    let _ = serve(stream, from, &device, &report);
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(50)),
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
}

struct Request {
    method: String,
    path: String,
    host: Option<String>,
    body: String,
}

fn serve(mut stream: TcpStream, from: SocketAddr, device: &Device, report: &Report) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let Some(request) = read_request(&mut stream)? else { return Ok(()) };
    let body: String = request.body.chars().take(512).collect();
    report(Event::Request { from, method: request.method.clone(), path: request.path.clone(), body });
    let host = request.host.clone().unwrap_or_else(|| {
        let ip = local_ip_towards(from).map_or_else(|| "127.0.0.1".to_string(), |ip| ip.to_string());
        format!("{ip}:{HTTP_PORT}")
    });
    let response = respond(&request, &host, device, report);
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

/// Reads a request: its head, and the body the head's `Content-Length` says (at most `MAX_REQUEST` altogether).
fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(at) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            break at;
        }
        if data.len() >= MAX_REQUEST {
            return Ok(None);
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(None);
        }
        data.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let (Some(method), Some(path)) = (first.next(), first.next()) else { return Ok(None) };
    let (mut host, mut length) = (None, 0usize);
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "host" => host = Some(value.trim().to_string()),
                "content-length" => length = value.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let length = length.min(MAX_REQUEST);
    let body_start = head_end + 4;
    while data.len() < body_start + length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let body_end = (body_start + length).min(data.len());
    let body = String::from_utf8_lossy(&data[body_start.min(body_end)..body_end]).into_owned();
    Ok(Some(Request { method: method.to_ascii_uppercase(), path: path.to_string(), host, body }))
}

/// The answer to one request.
fn respond(request: &Request, host: &str, device: &Device, report: &Report) -> String {
    let path = request.path.split('?').next().unwrap_or("");
    let app_path = format!("/apps/{APP}");
    match (request.method.as_str(), path) {
        ("GET" | "HEAD", "/dd.xml") => {
            let body = description(device);
            reply("200 OK", &[("Content-Type", "text/xml; charset=utf-8"), ("Application-URL", &format!("http://{host}/apps/"))], &body)
        }
        ("GET" | "HEAD", path) if path == app_path => {
            let running = device.running.lock().is_ok_and(|running| *running);
            reply("200 OK", &[("Content-Type", "text/xml; charset=utf-8")], &app_document(running))
        }
        ("POST", path) if path == app_path => {
            if let Ok(mut running) = device.running.lock() {
                *running = true;
            }
            report(Event::Launch { app: APP.to_string(), body: request.body.chars().take(512).collect() });
            reply("201 Created", &[("Location", &format!("http://{host}{app_path}/run"))], "")
        }
        ("DELETE", path) if path == format!("{app_path}/run") => {
            if let Ok(mut running) = device.running.lock() {
                *running = false;
            }
            report(Event::Stop { app: APP.to_string() });
            reply("200 OK", &[], "")
        }
        ("OPTIONS", _) => reply("200 OK", &[("Allow", "GET, HEAD, POST, DELETE, OPTIONS")], ""),
        _ => reply("404 Not Found", &[], ""),
    }
}

fn reply(status: &str, headers: &[(&str, &str)], body: &str) -> String {
    let mut text = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
    text
}

fn description(device: &Device) -> String {
    format!(
        "<?xml version=\"1.0\"?>\n<root xmlns=\"urn:schemas-upnp-org:device-1-0\" xmlns:r=\"urn:restful-tv-org:schemas:upnp-dd\">\
         <specVersion><major>1</major><minor>0</minor></specVersion><device>\
         <deviceType>urn:schemas-upnp-org:device:tvdevice:1</deviceType><friendlyName>{}</friendlyName>\
         <manufacturer>WestPlay</manufacturer><modelName>WestPlay Lite</modelName><UDN>uuid:{}</UDN></device></root>",
        escape(&device.name),
        device.uuid
    )
}

fn app_document(running: bool) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<service xmlns=\"urn:dial-multiscreen-org:schemas:dial\" dialVer=\"1.7\">\
         <name>{APP}</name><options allowStop=\"true\"/><state>{}</state>{}</service>",
        if running { "running" } else { "stopped" },
        if running { "<link rel=\"run\" href=\"run\"/>" } else { "" }
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A device id that stays the same for this computer and name (phones remember a device by it), in a UUID's shape.
fn uuid_for(name: &str) -> String {
    let seed = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    let fnv = |salt: u64| {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ salt;
        for byte in seed.bytes().chain([0]).chain(name.bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    };
    let (high, low) = (fnv(1), fnv(2));
    let (a, b, c, d, e) = (
        (high >> 32) as u32,
        (high >> 16) as u16,
        ((high as u16) & 0x0fff) | 0x5000,
        (((low >> 48) as u16) & 0x3fff) | 0x8000,
        low & 0xffff_ffff_ffff,
    );
    format!("{a:08x}-{b:04x}-{c:04x}-{d:04x}-{e:012x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn device() -> Arc<Device> {
        Arc::new(Device { name: "WestPlay".into(), uuid: uuid_for("WestPlay"), running: Mutex::new(false) })
    }

    #[test]
    fn the_device_id_is_a_stable_uuid() {
        let id = uuid_for("WestPlay");
        assert_eq!(id, uuid_for("WestPlay"));
        assert_ne!(id, uuid_for("Another"));
        let parts: Vec<_> = id.split('-').map(str::len).collect();
        assert_eq!(parts, [8, 4, 4, 4, 12]);
    }

    #[test]
    fn searches_are_read_and_the_right_ones_answered() {
        let dial = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:dial-multiscreen-org:service:dial:1\r\n\r\n";
        assert_eq!(search_target(dial).as_deref(), Some(DIAL_TARGET));
        let lower = "M-SEARCH * HTTP/1.1\r\nst: ssdp:all\r\n\r\n";
        assert_eq!(search_target(lower).as_deref(), Some("ssdp:all"));
        assert_eq!(search_target("NOTIFY * HTTP/1.1\r\nST: x\r\n\r\n"), None);
        assert_eq!(search_target(""), None);
        assert!(answers(DIAL_TARGET) && answers("ssdp:all") && answers("upnp:rootdevice"));
        assert!(!answers("urn:schemas-upnp-org:device:MediaRenderer:1"));
    }

    #[test]
    fn the_search_reply_says_where_the_description_is() {
        let reply = search_reply(&device(), Ipv4Addr::new(192, 168, 1, 20), 56790);
        assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(reply.contains("LOCATION: http://192.168.1.20:56790/dd.xml\r\n"));
        assert!(reply.contains(&format!("ST: {DIAL_TARGET}\r\n")));
        assert!(reply.ends_with("\r\n\r\n"));
    }

    #[test]
    fn a_search_gets_its_answer_over_udp() {
        let responder = UdpSocket::bind("127.0.0.1:0").unwrap();
        responder.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        let address = responder.local_addr().unwrap();
        let (events, seen) = mpsc::channel();
        let report: Report = Arc::new(move |event| {
            let _ = events.send(event);
        });
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let (stop, device) = (stop.clone(), device());
            thread::spawn(move || ssdp_loop(&responder, 56790, &device, &report, &stop))
        };
        let phone = UdpSocket::bind("127.0.0.1:0").unwrap();
        phone.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        phone.send_to(b"M-SEARCH * HTTP/1.1\r\nMAN: \"ssdp:discover\"\r\nST: urn:dial-multiscreen-org:service:dial:1\r\n\r\n", address).unwrap();
        let mut buffer = [0u8; 1024];
        let (length, _) = phone.recv_from(&mut buffer).unwrap();
        let reply = String::from_utf8_lossy(&buffer[..length]).into_owned();
        assert!(reply.contains("LOCATION: http://127.0.0.1:56790/dd.xml"), "{reply}");
        // A search for something else: told, not answered.
        phone.send_to(b"M-SEARCH * HTTP/1.1\r\nST: urn:other:service:x:1\r\n\r\n", address).unwrap();
        let first = seen.recv_timeout(Duration::from_secs(3)).unwrap();
        let second = seen.recv_timeout(Duration::from_secs(3)).unwrap();
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        assert!(matches!(first, Event::Search { answered: true, .. }), "{first:?}");
        assert!(matches!(second, Event::Search { answered: false, .. }), "{second:?}");
    }

    fn call(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut text = String::new();
        let _ = stream.read_to_string(&mut text);
        text
    }

    #[test]
    fn a_phone_finds_the_app_and_starts_and_stops_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (events, seen) = mpsc::channel();
        let report: Report = Arc::new(move |event| {
            let _ = events.send(event);
        });
        let stop = Arc::new(AtomicBool::new(false));
        let device = device();
        let worker = {
            let (stop, device, report) = (stop.clone(), device.clone(), report.clone());
            thread::spawn(move || http_loop(&listener, &device, &report, &stop))
        };

        let description = call(port, &format!("GET /dd.xml HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"));
        assert!(description.starts_with("HTTP/1.1 200 OK"), "{description}");
        assert!(description.contains(&format!("Application-URL: http://127.0.0.1:{port}/apps/")));
        assert!(description.contains("<friendlyName>WestPlay</friendlyName>"));

        let app = call(port, &format!("GET /apps/YouTube HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"));
        assert!(app.contains("<state>stopped</state>"), "{app}");

        let body = "pairingCode=abc-123&v=dQw4w9WgXcQ&t=0";
        let launch = call(
            port,
            &format!("POST /apps/YouTube HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: {}\r\n\r\n{body}", body.len()),
        );
        assert!(launch.starts_with("HTTP/1.1 201 Created"), "{launch}");
        assert!(launch.contains(&format!("Location: http://127.0.0.1:{port}/apps/YouTube/run")));

        let running = call(port, &format!("GET /apps/YouTube HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"));
        assert!(running.contains("<state>running</state>"), "{running}");

        let stopped = call(port, &format!("DELETE /apps/YouTube/run HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"));
        assert!(stopped.starts_with("HTTP/1.1 200 OK"), "{stopped}");
        let missing = call(port, &format!("GET /apps/Netflix HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"));
        assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");

        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        let all: Vec<Event> = seen.try_iter().collect();
        assert!(all.contains(&Event::Launch { app: "YouTube".into(), body: body.into() }), "{all:?}");
        assert!(all.contains(&Event::Stop { app: "YouTube".into() }), "{all:?}");
        assert!(all.iter().any(|event| matches!(event, Event::Request { method, path, .. } if method == "GET" && path == "/dd.xml")));
    }
}
