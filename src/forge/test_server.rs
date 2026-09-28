use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    headers: Vec<(String, String)>,
    pub body: String,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|e| panic!("request body is not JSON ({e}): {}", self.body))
    }
}

pub struct Reply {
    status: u16,
    body: String,
}

impl Reply {
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }

    pub fn raw(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

pub struct FakeServer {
    base: String,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl FakeServer {
    pub fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake server");
        let base = format!("http://{}", listener.local_addr().expect("local addr"));
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&recorded);
        std::thread::spawn(move || {
            for reply in replies {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                serve(stream, &reply, &sink);
            }
        });
        Self { base, recorded }
    }

    pub fn url(&self) -> &str {
        &self.base
    }

    pub fn requests(&self) -> std::sync::MutexGuard<'_, Vec<Recorded>> {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn only_request(&self) -> Recorded {
        let requests = self.requests();
        assert_eq!(requests.len(), 1, "expected exactly one request");
        requests[0].clone()
    }
}

fn serve(stream: TcpStream, reply: &Reply, sink: &Mutex<Vec<Recorded>>) {
    let mut reader = BufReader::new(stream);
    let request = read_request(&mut reader);
    sink.lock().unwrap_or_else(|e| e.into_inner()).push(request);
    let response = format!(
        "HTTP/1.1 {} Fake\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        reply.status,
        reply.body.len(),
        reply.body
    );
    let mut stream = reader.into_inner();
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn read_request(reader: &mut BufReader<TcpStream>) -> Recorded {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).expect("request line");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }

    let length = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).expect("request body");

    Recorded {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}
