// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A WebDriver session in headless Firefox through geckodriver, with only
//! the commands the UI smoke test sends: navigate, find, type, click and
//! read the URL.
//!
//! [`Session::start`] runs `$GECKODRIVER`, else `/snap/bin/geckodriver`,
//! else `geckodriver` on PATH, on ports the OS picked, and skips with a
//! message when none runs. geckodriver creates the Firefox profile; a snap
//! geckodriver shares the snap Firefox's private `/tmp`. Firefox's output,
//! the pages' console included, goes to geckodriver's, which is captured
//! and printed when the test fails.
//!
//! Each command is one HTTP/1.1 request on its own connection.

use std::io::{BufRead, BufReader, Read};
use std::net::{SocketAddr, TcpListener as StdListener};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// How long geckodriver has to listen, and Firefox to start.
const START_TIMEOUT: Duration = Duration::from_secs(60);
/// The W3C id of an element in a command's answer.
const ELEMENT: &str = "element-6066-11e4-a52e-4f735466cecf";

/// A port nothing listens on, for a child process to bind.
fn free_port() -> u16 {
    StdListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn geckodriver() -> Option<String> {
    let candidates = std::env::var("GECKODRIVER")
        .ok()
        .into_iter()
        .chain(["/snap/bin/geckodriver".to_owned(), "geckodriver".to_owned()]);
    for c in candidates {
        let runs = Command::new(&c)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if runs {
            return Some(c);
        }
    }
    None
}

/// A geckodriver and its one session. Dropping it ends the session and
/// kills geckodriver.
pub struct Session {
    addr: SocketAddr,
    id: String,
    driver: Child,
    output: Arc<Mutex<Vec<String>>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.driver.kill();
        let _ = self.driver.wait();
        if std::thread::panicking() {
            eprintln!(
                "--- geckodriver output ---\n{}\n--- end of geckodriver output ---",
                self.output().join("\n")
            );
        }
    }
}

/// An HTTP answer's status and JSON body.
async fn request(addr: SocketAddr, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut conn = TcpStream::connect(addr).await.unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    conn.write_all(head.as_bytes()).await.unwrap();
    conn.write_all(body.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    conn.read_to_end(&mut raw).await.unwrap();
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut res = httparse::Response::new(&mut headers);
    let Ok(httparse::Status::Complete(len)) = res.parse(&raw) else {
        panic!("geckodriver's answer to {method} {path} is cut short");
    };
    let status = res.code.unwrap();
    let value = serde_json::from_slice(&raw[len..]).unwrap_or(Value::Null);
    (status, value)
}

impl Session {
    /// A session in a new headless Firefox with `prefs`. `None` after
    /// printing why geckodriver can't run here.
    pub async fn start(prefs: Value) -> Option<Session> {
        let Some(bin) = geckodriver() else {
            eprintln!("skipping: no geckodriver (set GECKODRIVER)");
            return None;
        };
        let port = free_port();
        let mut driver = Command::new(bin)
            .args(["--port", &port.to_string()])
            .args(["--websocket-port", &free_port().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = Arc::<Mutex<Vec<String>>>::default();
        let pipes: [Box<dyn Read + Send>; 2] = [
            Box::new(driver.stdout.take().unwrap()),
            Box::new(driver.stderr.take().unwrap()),
        ];
        for pipe in pipes {
            let output = output.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    output.lock().unwrap().push(line);
                }
            });
        }
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let deadline = tokio::time::Instant::now() + START_TIMEOUT;
        while TcpStream::connect(addr).await.is_err() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "geckodriver didn't listen on {addr}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let mut session = Session {
            addr,
            id: String::new(),
            driver,
            output,
        };
        let caps = json!({
            "capabilities": {
                "alwaysMatch": {
                    "browserName": "firefox",
                    "moz:firefoxOptions": {"args": ["-headless"], "prefs": prefs},
                },
            },
        });
        let v = session.command("POST", "/session", Some(&caps)).await;
        session.id = v["sessionId"].as_str().unwrap().to_owned();
        Some(session)
    }

    /// What geckodriver and Firefox printed so far.
    pub fn output(&self) -> Vec<String> {
        self.output.lock().map(|o| o.clone()).unwrap_or_default()
    }

    /// A command's `value`, failing the test on an error answer.
    async fn command(&self, method: &str, path: &str, body: Option<&Value>) -> Value {
        match self.try_command(method, path, body).await {
            Ok(v) => v,
            Err(e) => panic!("{method} {path}: {e}"),
        }
    }

    async fn try_command(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, Value> {
        let (status, mut v) = request(self.addr, method, path, body).await;
        let value = v["value"].take();
        if status == 200 { Ok(value) } else { Err(value) }
    }

    fn path(&self, rest: &str) -> String {
        format!("/session/{}{rest}", self.id)
    }

    pub async fn navigate(&self, url: &str) {
        self.command("POST", &self.path("/url"), Some(&json!({"url": url})))
            .await;
    }

    pub async fn url(&self) -> String {
        let v = self.command("GET", &self.path("/url"), None).await;
        v.as_str().unwrap().to_owned()
    }

    /// The first element `css` selects, if one does.
    pub async fn find(&self, css: &str) -> Option<String> {
        let by = json!({"using": "css selector", "value": css});
        let v = self
            .try_command("POST", &self.path("/element"), Some(&by))
            .await
            .ok()?;
        Some(v[ELEMENT].as_str()?.to_owned())
    }

    /// The first element `css` selects, waiting up to `timeout` for one.
    pub async fn wait_for(&self, css: &str, timeout: Duration) -> String {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let Some(e) = self.find(css).await {
                return e;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out after {timeout:?} waiting for {css}; at {}",
                self.url().await
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn type_into(&self, element: &str, text: &str) {
        let path = self.path(&format!("/element/{element}/value"));
        self.command("POST", &path, Some(&json!({"text": text})))
            .await;
    }

    pub async fn click(&self, element: &str) {
        let path = self.path(&format!("/element/{element}/click"));
        self.command("POST", &path, Some(&json!({}))).await;
    }

    /// Ends the session, which closes Firefox.
    pub async fn end(self) {
        let _ = self.try_command("DELETE", &self.path(""), None).await;
    }
}
