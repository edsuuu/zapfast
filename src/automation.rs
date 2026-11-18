//! Agent access: lets a local agent, such as Claude Code through
//! `zapfast mcp`, list chats, read messages and send text with the account
//! ZapFast is linked to. Off by default (Settings > System > Agent access).
//!
//! While it is on, ZapFast listens on a loopback port and writes the port
//! and a random token to `automation.json` in the per-user runtime
//! directory, readable only by the user. Every request must carry that
//! token. A request is one line of JSON and gets one line back; the
//! listener only queues it for the app, which answers it on its next frame,
//! or hands it to the account's worker, which answers it directly.
//!
//! Nothing here logs what is read or sent: no text, numbers or chat names.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The file in the runtime directory that says where to connect.
pub const ENDPOINT_FILE: &str = "automation.json";
/// Longest request line accepted: a long message and its envelope.
const REQUEST_LIMIT: u64 = 256 * 1024;
/// Time a client gets to send its request.
const REQUEST_TIME: Duration = Duration::from_secs(5);
/// Time the app gets to answer: sending to a number first asks WhatsApp
/// whether the number has an account.
pub const ANSWER_TIME: Duration = Duration::from_secs(30);
/// Most messages one read returns.
pub const MAX_MESSAGES: usize = 200;
/// Most chats one listing returns.
pub const MAX_CHATS: usize = 500;

/// Where a running ZapFast listens, as `automation.json` says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

impl Endpoint {
    /// Reads the endpoint ZapFast wrote in `runtime`, if it is listening.
    pub fn read(runtime: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(runtime.join(ENDPOINT_FILE)).ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// What an agent asks of ZapFast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    /// The linked account and whether it is connected.
    Status,
    /// Chats, most recent first. Locked chats are never listed.
    ListChats {
        #[serde(default)]
        query: Option<String>,
        #[serde(default)]
        unread_only: bool,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// The latest messages of a chat, oldest first. `chat` is a chat id
    /// or a phone number with a chat already.
    ReadMessages {
        chat: String,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// Sends text to a chat id, or to a phone number in international form,
    /// which need not have a chat yet.
    SendMessage { to: String, text: String },
}

/// What the app answers: a JSON result, or an error for the agent to read.
pub type Reply = Result<Value, String>;

/// One line on the wire: the token and the request.
#[derive(Debug, Serialize, Deserialize)]
pub struct Envelope {
    pub token: String,
    pub request: Request,
}

/// A request waiting for the app, with the way back to the agent.
#[derive(Debug)]
pub struct Pending {
    pub request: Request,
    pub reply: mpsc::Sender<Reply>,
}

/// Requests queued by the listener, drained by the app every frame.
pub type Queue = Arc<Mutex<Vec<Pending>>>;

/// What the app asks of an account's worker, which answers on `reply`.
#[derive(Debug)]
pub enum WorkerRequest {
    /// The latest `limit` messages of `chat`, oldest first.
    Read {
        chat: String,
        limit: usize,
        reply: mpsc::Sender<Reply>,
    },
    /// Text for `to`: a chat the app found, or a number WhatsApp is asked
    /// about first.
    Send {
        to: Recipient,
        text: String,
        reply: mpsc::Sender<Reply>,
    },
}

/// Whom a message goes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recipient {
    Chat(String),
    /// Digits only, in international form.
    Phone(String),
}

/// The loopback listener, while agent access is on.
pub struct Server {
    stop: Arc<AtomicBool>,
    port: u16,
    file: PathBuf,
    token: String,
}

impl Server {
    /// Listens on a free loopback port and publishes it, with a new token,
    /// in `runtime`. Requests land in `queue`, and `wake` is called for each.
    pub fn start(
        runtime: &Path,
        queue: Queue,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let token = new_token()?;
        let file = runtime.join(ENDPOINT_FILE);
        let endpoint = Endpoint {
            port,
            token: token.clone(),
            pid: std::process::id(),
        };
        write_private(&file, &serde_json::to_vec(&endpoint)?)?;
        let stop = Arc::new(AtomicBool::new(false));
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let (stopping, expected) = (Arc::clone(&stop), token.clone());
        std::thread::Builder::new()
            .name("zapfast-automation".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopping.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let (queue, wake, expected) =
                        (Arc::clone(&queue), Arc::clone(&wake), expected.clone());
                    let _ = std::thread::Builder::new()
                        .name("zapfast-automation-request".into())
                        .spawn(move || serve(stream, &expected, &queue, &*wake));
                }
            })?;
        log::info!("agent access is listening on the loopback interface");
        Ok(Self {
            stop,
            port,
            file,
            token,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Wake the accept loop so it sees the flag.
        let _ = TcpStream::connect_timeout(
            &(Ipv4Addr::LOCALHOST, self.port).into(),
            Duration::from_millis(200),
        );
        // Remove the file only while it is still this listener's: a later
        // copy may have written its own.
        if Endpoint::read(self.file.parent().unwrap_or(Path::new(".")))
            .is_some_and(|endpoint| endpoint.token == self.token)
        {
            let _ = std::fs::remove_file(&self.file);
        }
        log::info!("agent access stopped");
    }
}

/// A random 256-bit token, as hex.
fn new_token() -> std::io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Writes `bytes` to `path` readable only by the user, replacing it whole.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("json.new");
    let _ = std::fs::remove_file(&temporary);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path)
}

/// Compares tokens in time that does not depend on where they differ.
fn same_token(given: &str, expected: &str) -> bool {
    given.len() == expected.len()
        && given
            .bytes()
            .zip(expected.bytes())
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

/// Answers one connection: one request line, one reply line.
fn serve(stream: TcpStream, expected: &str, queue: &Queue, wake: &(dyn Fn() + Send + Sync)) {
    let _ = stream.set_read_timeout(Some(REQUEST_TIME));
    let mut line = String::new();
    let read = BufReader::new((&stream).take(REQUEST_LIMIT)).read_line(&mut line);
    let reply = match read {
        Ok(0) | Err(_) => return,
        Ok(_) => answer(&line, expected, queue, wake),
    };
    let mut out = match reply {
        Ok(result) => json!({ "ok": result }),
        Err(error) => json!({ "error": error }),
    }
    .to_string();
    out.push('\n');
    let mut stream = stream;
    let _ = stream.write_all(out.as_bytes());
}

fn answer(line: &str, expected: &str, queue: &Queue, wake: &(dyn Fn() + Send + Sync)) -> Reply {
    let envelope: Envelope = serde_json::from_str(line.trim())
        .map_err(|_| "ZapFast could not read the request".to_owned())?;
    if !same_token(&envelope.token, expected) {
        return Err("The agent access token is wrong; restart `zapfast mcp`".to_owned());
    }
    let (reply, answered) = mpsc::channel();
    queue
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(Pending {
            request: envelope.request,
            reply,
        });
    wake();
    match answered.recv_timeout(ANSWER_TIME) {
        Ok(reply) => reply,
        Err(mpsc::RecvTimeoutError::Timeout) => Err("ZapFast took too long to answer".to_owned()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("ZapFast could not answer; is the account connected?".to_owned())
        }
    }
}

/// Sends `request` to the ZapFast listening in `runtime` and waits for its
/// answer.
pub fn call(runtime: &Path, request: &Request) -> Reply {
    let endpoint = Endpoint::read(runtime).ok_or_else(|| {
        "ZapFast is not running, or agent access is off. Open ZapFast and turn on \
         Settings > System > Agent access (MCP)."
            .to_owned()
    })?;
    call_endpoint(&endpoint, request)
}

/// [`call`] against a known endpoint.
pub fn call_endpoint(endpoint: &Endpoint, request: &Request) -> Reply {
    let unreachable = |_| "Could not reach ZapFast. Is it running with agent access on?".to_owned();
    let mut stream = TcpStream::connect_timeout(
        &(Ipv4Addr::LOCALHOST, endpoint.port).into(),
        Duration::from_secs(2),
    )
    .map_err(unreachable)?;
    let _ = stream.set_read_timeout(Some(ANSWER_TIME + Duration::from_secs(5)));
    let mut line = serde_json::to_string(&Envelope {
        token: endpoint.token.clone(),
        request: request.clone(),
    })
    .map_err(|error| error.to_string())?;
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(unreachable)?;
    let mut answer = String::new();
    BufReader::new(stream)
        .read_line(&mut answer)
        .map_err(unreachable)?;
    let answer: Value = serde_json::from_str(answer.trim())
        .map_err(|_| "ZapFast closed the connection without answering".to_owned())?;
    match (answer.get("ok"), answer.get("error")) {
        (Some(result), _) => Ok(result.clone()),
        (None, Some(Value::String(error))) => Err(error.clone()),
        _ => Err("ZapFast answered something unexpected".to_owned()),
    }
}

/// A message as an agent reads it.
pub fn message_json(message: &crate::model::Message) -> Value {
    let mut value = json!({
        "id": message.id,
        "from_me": message.from_me,
        "sender": message.sender,
        "timestamp": message.timestamp,
        "text": message.content.full_summary(),
        "status": format!("{:?}", message.status).to_lowercase(),
    });
    if let Some(name) = message
        .sender_name
        .as_deref()
        .filter(|name| !name.is_empty())
    {
        value["sender_name"] = json!(name);
    }
    if let Some(quoted) = &message.quoted {
        value["reply_to"] = json!({ "id": quoted.id, "text": quoted.summary });
    }
    if message.edited {
        value["edited"] = json!(true);
    }
    value
}

/// Phone numbers a chat with `digits` may be stored under. Many Brazilian
/// mobiles are registered without the ninth digit their numbers gained
/// later, so a number is also tried with it dropped, or added.
pub fn phone_forms(digits: &str) -> Vec<String> {
    let mut forms = vec![digits.to_owned()];
    if let Some(rest) = digits.strip_prefix("55") {
        match rest.len() {
            // Area code, 9, eight digits: also without the 9.
            11 if rest.as_bytes()[2] == b'9' => {
                forms.push(format!("55{}{}", &rest[..2], &rest[3..]));
            }
            // Area code and eight digits of a mobile: also with the 9.
            10 if matches!(rest.as_bytes()[2], b'6'..=b'9') => {
                forms.push(format!("55{}9{}", &rest[..2], &rest[2..]));
            }
            _ => {}
        }
    }
    forms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_read_as_the_wire_has_them() {
        let request: Request = serde_json::from_str(
            r#"{"method":"send_message","params":{"to":"5511987654321","text":"oi"}}"#,
        )
        .unwrap();
        assert_eq!(
            request,
            Request::SendMessage {
                to: "5511987654321".into(),
                text: "oi".into()
            }
        );
        let request: Request = serde_json::from_str(r#"{"method":"status"}"#).unwrap();
        assert_eq!(request, Request::Status);
        let request: Request =
            serde_json::from_str(r#"{"method":"list_chats","params":{}}"#).unwrap();
        assert_eq!(
            request,
            Request::ListChats {
                query: None,
                unread_only: false,
                limit: None
            }
        );
    }

    #[test]
    fn tokens_compare_whole() {
        assert!(same_token("abc", "abc"));
        assert!(!same_token("abd", "abc"));
        assert!(!same_token("ab", "abc"));
        assert!(!same_token("", "abc"));
    }

    #[test]
    fn brazilian_mobiles_are_tried_with_and_without_the_ninth_digit() {
        assert_eq!(
            phone_forms("5511987654321"),
            ["5511987654321", "551187654321"]
        );
        assert_eq!(
            phone_forms("551187654321"),
            ["551187654321", "5511987654321"]
        );
        // A landline (starting 2 to 5) has no ninth digit.
        assert_eq!(phone_forms("551133334444"), ["551133334444"]);
        assert_eq!(phone_forms("4915112345678"), ["4915112345678"]);
    }

    /// A request goes through the listener to the queue, and the answer
    /// comes back; a wrong token never reaches the queue.
    #[test]
    fn the_listener_hands_requests_to_the_app_and_answers() {
        let dir = tempfile::tempdir().unwrap();
        let queue = Queue::default();
        let server = Server::start(dir.path(), Arc::clone(&queue), || {}).unwrap();
        let endpoint = Endpoint::read(dir.path()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join(ENDPOINT_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let app = {
            let queue = Arc::clone(&queue);
            std::thread::spawn(move || {
                loop {
                    let pending = queue.lock().unwrap().pop();
                    if let Some(pending) = pending {
                        assert_eq!(pending.request, Request::Status);
                        pending
                            .reply
                            .send(Ok(json!({ "connected": true })))
                            .unwrap();
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        };
        assert_eq!(
            call(dir.path(), &Request::Status),
            Ok(json!({ "connected": true }))
        );
        app.join().unwrap();
        let wrong = Endpoint {
            token: "0".repeat(64),
            ..endpoint
        };
        assert!(call_endpoint(&wrong, &Request::Status).is_err());
        assert!(queue.lock().unwrap().is_empty());
        // Stopping removes the endpoint file.
        drop(server);
        assert!(Endpoint::read(dir.path()).is_none());
        assert!(call(dir.path(), &Request::Status).is_err());
    }

    /// A request the app drops, as an offline backend does, is answered
    /// at once rather than after the timeout.
    #[test]
    fn a_dropped_request_is_answered_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let queue = Queue::default();
        let _server = Server::start(dir.path(), Arc::clone(&queue), || {}).unwrap();
        let dropper = {
            let queue = Arc::clone(&queue);
            std::thread::spawn(move || {
                loop {
                    if queue.lock().unwrap().pop().is_some() {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        };
        let started = std::time::Instant::now();
        assert!(call(dir.path(), &Request::Status).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        dropper.join().unwrap();
    }
}
