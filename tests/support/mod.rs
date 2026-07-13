use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

pub mod process;

pub struct FixtureRegistry {
    base_url: String,
}

impl FixtureRegistry {
    pub fn start() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture registry");
        let addr = listener.local_addr().expect("fixture registry address");
        let base_url = format!("http://{}", addr);
        let thread_base_url = base_url.clone();

        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                handle_connection(stream, &root, &thread_base_url);
            }
        });

        Self { base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

fn handle_connection(mut stream: TcpStream, root: &Path, base_url: &str) {
    let mut buffer = [0_u8; 4096];
    let Ok(read) = stream.read(&mut buffer) else {
        return;
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let Some(line) = request.lines().next() else {
        return;
    };
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");

    if !matches!(method, "GET" | "HEAD") {
        write_response(&mut stream, "405 Method Not Allowed", "text/plain", b"");
        return;
    }

    match fixture_bytes(root, path, base_url) {
        Some((content_type, bytes)) if method == "HEAD" => {
            write_head_response(&mut stream, content_type, bytes.len())
        }
        Some((content_type, bytes)) => write_response(&mut stream, "200 OK", content_type, &bytes),
        None => write_response(&mut stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn fixture_bytes(root: &Path, path: &str, base_url: &str) -> Option<(&'static str, Vec<u8>)> {
    let relative = path.trim_start_matches('/');
    let file = match relative {
        "formula.json" => root.join("formula.json"),
        p if p.starts_with("formula/") && p.ends_with(".json") => root.join(p),
        p if p.starts_with("bottles/") => root.join(p),
        _ => return None,
    };

    let mut bytes = fs::read(file).ok()?;
    let content_type = if relative.ends_with(".json") {
        let text = String::from_utf8(bytes)
            .ok()?
            .replace("http://127.0.0.1:0", base_url);
        bytes = text.into_bytes();
        "application/json"
    } else {
        "application/octet-stream"
    };

    Some((content_type, bytes))
}

fn write_head_response(stream: &mut TcpStream, content_type: &str, content_length: usize) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        content_type, content_length
    );
    let _ = stream.write_all(header.as_bytes());
}

fn write_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}
