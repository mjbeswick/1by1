#![allow(dead_code)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

/// Remote commands accepted by the web interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remote {
    Resume,
    Prev,
    Next,
    Rewind,
    Forward,
    VolumeDown,
    VolumeUp,
    DirBack,
    DirNext,
    Home,
    Stop,
}

pub struct HttpServer {
    pub addr: String,
    stop: Arc<AtomicBool>,
    rx: Receiver<Remote>,
    tx: Sender<Remote>,
    handle: Option<thread::JoinHandle<()>>,
}

impl HttpServer {
    pub fn start(addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(addr).map_err(|e| format!("bind {addr}: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let (ctx, _crx) = channel::<()>();
        let stop2 = stop.clone();
        let tx2 = tx.clone();
        let handle = thread::spawn(move || {
            while !stop2.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_client(stream, &tx2);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(120));
                    }
                    Err(_) => break,
                }
            }
        });
        let _ = ctx;
        Ok(Self {
            addr: addr.to_string(),
            stop,
            rx,
            tx,
            handle: Some(handle),
        })
    }

    pub fn poll(&self) -> Vec<Remote> {
        let mut out = Vec::new();
        while let Ok(c) = self.rx.try_recv() {
            out.push(c);
        }
        out
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    pub fn sender(&self) -> Sender<Remote> {
        self.tx.clone()
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_client(mut stream: TcpStream, tx: &Sender<Remote>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(400)))?;
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();

    if path == "/1by1.m3u" {
        let body = "#EXTM3U\n#EXTINF:-1,1by1 stream\nhttp://127.0.0.1/1by1.mp3\n";
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: audio/x-mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(body.as_bytes())?;
        return Ok(());
    }

    let cmd = match path.as_str() {
        "/res" => Some(Remote::Resume),
        "/prv" => Some(Remote::Prev),
        "/nxt" => Some(Remote::Next),
        "/rew" => Some(Remote::Rewind),
        "/fwd" => Some(Remote::Forward),
        "/vdn" => Some(Remote::VolumeDown),
        "/vup" => Some(Remote::VolumeUp),
        "/dprv" => Some(Remote::DirBack),
        "/dnxt" => Some(Remote::DirNext),
        "/home" => Some(Remote::Home),
        _ => None,
    };
    if let Some(c) = cmd {
        let _ = tx.send(c);
    }

    let body = page();
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

fn page() -> String {
    let mut s = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>1by1</title>\
         <style>body{font-family:system-ui,sans-serif;background:#111;color:#eee;margin:0;padding:24px}\
         a{display:block;padding:14px 18px;margin:8px 0;background:#222;color:#eee;text-decoration:none;border-radius:6px}\
         a:hover{background:#333}h1{font-size:18px;font-weight:600}</style></head><body><h1>1by1 remote</h1>",
    );
    for (path, label) in [
        ("/res", "Stop / Resume"),
        ("/prv", "Previous track"),
        ("/nxt", "Next track"),
        ("/rew", "Rewind"),
        ("/fwd", "Forward"),
        ("/vdn", "Volume down"),
        ("/vup", "Volume up"),
        ("/dprv", "Previous folder"),
        ("/dnxt", "Next folder"),
        ("/home", "Go to current track"),
    ] {
        s.push_str(&format!("<a href=\"{path}\">{label}</a>"));
    }
    s.push_str("</body></html>");
    s
}
