//! `zapfast mcp`: a Model Context Protocol server on stdin and stdout, so an
//! agent such as Claude Code can use the WhatsApp account linked in the
//! running ZapFast: list chats, read them, send text, and wait for an
//! answer, which is what testing a chatbot takes.
//!
//! It holds no WhatsApp session of its own. Each tool call is passed to the
//! running ZapFast over its agent access channel (see [`crate::automation`]),
//! which must be turned on in Settings > System. Only JSON-RPC goes to
//! stdout; nothing is logged.
//!
//! ```sh
//! claude mcp add zapfast -- zapfast mcp
//! ```

use std::io::{BufRead, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::automation::{self, Reply, Request};

/// Protocol revisions this server speaks, newest first.
const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
/// How often `wait_for_reply` looks for new messages.
const POLL: Duration = Duration::from_secs(1);
/// How many recent messages `wait_for_reply` looks through.
const WINDOW: usize = 100;

const INSTRUCTIONS: &str = "Tools for the WhatsApp account linked in ZapFast on this \
computer. Messages sent with send_message are real and go out as the user, so send only \
what the user asked for. To test a chatbot: send_message to its number, then \
wait_for_reply on the chat id it returns.";

/// Serves MCP on stdin and stdout until stdin closes; returns the exit code.
pub fn serve(runtime: &Path) -> i32 {
    let call = |request: &Request| automation::call(runtime, request);
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_line(&line, &call, POLL)
            && (writeln!(stdout, "{response}").is_err() || stdout.flush().is_err())
        {
            break;
        }
    }
    0
}

/// Answers one JSON-RPC message; `None` for a notification or a response,
/// which take no answer.
pub fn handle_line(line: &str, call: &dyn Fn(&Request) -> Reply, poll: Duration) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(rpc_error(Value::Null, -32700, "Parse error"));
    };
    let id = message.get("id").cloned();
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response to a request this server never makes.
        return None;
    };
    let id = id?;
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => tools_call(&params, call, poll),
        _ => Err((-32601, format!("Method not found: {method}"))),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => rpc_error(id, code, &message),
    })
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|asked| PROTOCOL_VERSIONS.contains(asked))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": "zapfast",
            "title": "ZapFast",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    let chat = json!({
        "type": "string",
        "description": "A chat id from list_chats (such as 5511987654321@s.whatsapp.net, \
                        or a group's ...@g.us), or a phone number in international form, \
                        digits only, country code first (such as 5511987654321).",
    });
    json!([
        {
            "name": "whatsapp_status",
            "title": "WhatsApp status",
            "description": "Whether ZapFast is connected to WhatsApp, and the number and \
                            name of the account it is linked to.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "list_chats",
            "title": "List chats",
            "description": "WhatsApp chats, most recent first, with their id, name, kind \
                            (direct, group or channel), unread count and last message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Only chats whose name or id contains this.",
                    },
                    "unread_only": { "type": "boolean", "description": "Only unread chats." },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": automation::MAX_CHATS,
                        "description": "How many chats, 50 if left out.",
                    },
                },
            },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "read_messages",
            "title": "Read messages",
            "description": "The latest messages of a chat, oldest first: id, from_me, \
                            sender, timestamp (Unix seconds), text and delivery status. \
                            Media shows as a label such as \"Photo\", with its caption.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chat": chat,
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": automation::MAX_MESSAGES,
                        "description": "How many messages, 20 if left out.",
                    },
                },
                "required": ["chat"],
            },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "send_message",
            "title": "Send a message",
            "description": "Sends a text message as the user. `to` is a chat id from \
                            list_chats or a phone number in international form; a number \
                            with no chat yet is first checked with WhatsApp, which also \
                            finds a Brazilian mobile registered without its ninth digit. \
                            Returns the chat id and the message id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": chat,
                    "text": {
                        "type": "string",
                        "description": "The message. WhatsApp formatting works: *bold*, \
                                        _italic_, ~strikethrough~.",
                    },
                },
                "required": ["to", "text"],
            },
            "annotations": {
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": true,
            },
        },
        {
            "name": "wait_for_reply",
            "title": "Wait for a reply",
            "description": "Waits for new incoming messages in a chat, such as a chatbot's \
                            answer after send_message, and returns them. Counts messages \
                            after `after`, or after the last message you sent in the chat, \
                            so an answer that came before the call is not missed; once one \
                            arrives it keeps collecting until none comes for \
                            settle_seconds, so an answer in several bubbles comes back \
                            whole.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chat": chat,
                    "after": {
                        "type": "string",
                        "description": "A message id: count only messages after it.",
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 600,
                        "description": "How long to wait for a first message, 60 if left out.",
                    },
                    "settle_seconds": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 30,
                        "description": "Quiet time that ends the answer, 3 if left out.",
                    },
                },
                "required": ["chat"],
            },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
    ])
}

/// Runs a tool. A failure the agent should read, such as ZapFast not
/// running, is a tool result with `isError`, not a protocol error.
fn tools_call(
    params: &Value,
    call: &dyn Fn(&Request) -> Reply,
    poll: Duration,
) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let number = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
    };
    let missing = |key: &str| Err(format!("`{key}` is required"));
    let outcome = match name {
        "whatsapp_status" => call(&Request::Status),
        "list_chats" => call(&Request::ListChats {
            query: text("query"),
            unread_only: arguments
                .get("unread_only")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            limit: number("limit"),
        }),
        "read_messages" => match text("chat") {
            Some(chat) => call(&Request::ReadMessages {
                chat,
                limit: number("limit"),
            }),
            None => missing("chat"),
        },
        "send_message" => match (text("to"), text("text")) {
            (Some(to), Some(text)) => call(&Request::SendMessage { to, text }),
            (None, _) => missing("to"),
            (_, None) => missing("text"),
        },
        "wait_for_reply" => match text("chat") {
            Some(chat) => wait_for_reply(
                call,
                &chat,
                text("after").as_deref(),
                Duration::from_secs(number("timeout_seconds").unwrap_or(60).clamp(1, 600) as u64),
                Duration::from_secs(number("settle_seconds").unwrap_or(3).min(30) as u64),
                poll,
            ),
            None => missing("chat"),
        },
        _ => return Err((-32602, format!("Unknown tool: {name}"))),
    };
    Ok(match outcome {
        Ok(value) => json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&value).unwrap_or_default(),
            }],
            "isError": false,
        }),
        Err(error) => json!({
            "content": [{ "type": "text", "text": error }],
            "isError": true,
        }),
    })
}

/// Polls the chat until incoming messages arrive after the anchor and then
/// stop for `settle`, or until `timeout` passes with none.
fn wait_for_reply(
    call: &dyn Fn(&Request) -> Reply,
    chat: &str,
    after: Option<&str>,
    timeout: Duration,
    settle: Duration,
    poll: Duration,
) -> Reply {
    let read = || -> Result<(String, Vec<Value>), String> {
        let answer = call(&Request::ReadMessages {
            chat: chat.to_owned(),
            limit: Some(WINDOW),
        })?;
        let id = answer
            .get("chat")
            .and_then(Value::as_str)
            .unwrap_or(chat)
            .to_owned();
        let messages = answer
            .get("messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok((id, messages))
    };
    let started = Instant::now();
    let (id, mut messages) = read()?;
    // Without `after`, what counts is what came after the last message sent
    // from here, or, when there is none, after the last one there was.
    let anchor = after.map(str::to_owned).or_else(|| {
        messages
            .iter()
            .rev()
            .find(|message| message["from_me"] == true)
            .or_else(|| messages.last())
            .and_then(|message| message["id"].as_str())
            .map(str::to_owned)
    });
    let mut seen = 0;
    let mut changed: Option<Instant> = None;
    loop {
        let replies = incoming_after(&messages, anchor.as_deref());
        if replies.len() != seen {
            seen = replies.len();
            changed = Some(Instant::now());
        }
        let settled = changed.is_some_and(|at| at.elapsed() >= settle);
        if settled || started.elapsed() >= timeout {
            return Ok(json!({
                "chat": id,
                "replies": replies,
                "timed_out": replies.is_empty(),
            }));
        }
        std::thread::sleep(poll);
        messages = read()?.1;
    }
}

/// The messages from others after the one with id `anchor`; all of them
/// when there is no anchor, or it has scrolled out of the window.
fn incoming_after(messages: &[Value], anchor: Option<&str>) -> Vec<Value> {
    let start = anchor
        .and_then(|anchor| {
            messages
                .iter()
                .position(|message| message["id"].as_str() == Some(anchor))
        })
        .map_or(0, |index| index + 1);
    messages[start..]
        .iter()
        .filter(|message| message["from_me"] == false)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn rpc(line: Value, call: &dyn Fn(&Request) -> Reply) -> Option<Value> {
        handle_line(&line.to_string(), call, Duration::from_millis(1))
    }

    fn offline(_: &Request) -> Reply {
        Err("ZapFast is not running".to_owned())
    }

    #[test]
    fn it_introduces_itself_in_the_clients_revision() {
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                   "params": {"protocolVersion": "2025-03-26", "capabilities": {},
                              "clientInfo": {"name": "test", "version": "0"}}}),
            &offline,
        )
        .unwrap();
        assert_eq!(answer["id"], 1);
        assert_eq!(answer["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(answer["result"]["serverInfo"]["name"], "zapfast");
        assert!(answer["result"]["capabilities"]["tools"].is_object());
        // An unknown revision gets the newest.
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 2, "method": "initialize",
                   "params": {"protocolVersion": "1999-01-01"}}),
            &offline,
        )
        .unwrap();
        assert_eq!(answer["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn notifications_get_no_answer_and_unknown_methods_an_error() {
        assert!(
            rpc(
                json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
                &offline
            )
            .is_none()
        );
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": "x", "method": "resources/list"}),
            &offline,
        )
        .unwrap();
        assert_eq!(answer["error"]["code"], -32601);
        let answer = handle_line("{not json", &offline, Duration::ZERO).unwrap();
        assert_eq!(answer["error"]["code"], -32700);
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}),
            &offline,
        )
        .unwrap();
        assert_eq!(answer["result"], json!({}));
    }

    #[test]
    fn every_tool_has_a_schema() {
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            &offline,
        )
        .unwrap();
        let tools = answer["result"]["tools"].as_array().unwrap();
        let names: Vec<_> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "whatsapp_status",
                "list_chats",
                "read_messages",
                "send_message",
                "wait_for_reply"
            ]
        );
        for tool in tools {
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
            assert!(
                tool["description"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            );
        }
    }

    #[test]
    fn tool_calls_become_requests_and_failures_are_tool_errors() {
        let asked = RefCell::new(Vec::new());
        let call = |request: &Request| {
            asked.borrow_mut().push(request.clone());
            Ok(json!({ "chat": "1@s.whatsapp.net", "id": "ABC" }))
        };
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
                   "params": {"name": "send_message",
                              "arguments": {"to": "5511987654321", "text": "oi"}}}),
            &call,
        )
        .unwrap();
        assert_eq!(answer["result"]["isError"], false);
        assert!(
            answer["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("ABC")
        );
        assert_eq!(
            asked.borrow().as_slice(),
            [Request::SendMessage {
                to: "5511987654321".into(),
                text: "oi".into()
            }]
        );
        // A missing argument and ZapFast being away are results the agent reads.
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call",
                   "params": {"name": "send_message", "arguments": {"to": "1"}}}),
            &call,
        )
        .unwrap();
        assert_eq!(answer["result"]["isError"], true);
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
                   "params": {"name": "whatsapp_status"}}),
            &offline,
        )
        .unwrap();
        assert_eq!(answer["result"]["isError"], true);
        assert_eq!(
            answer["result"]["content"][0]["text"],
            "ZapFast is not running"
        );
        // An unknown tool is a protocol error.
        let answer = rpc(
            json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call",
                   "params": {"name": "delete_everything"}}),
            &call,
        )
        .unwrap();
        assert_eq!(answer["error"]["code"], -32602);
    }

    fn message(id: &str, from_me: bool) -> Value {
        json!({ "id": id, "from_me": from_me, "text": id })
    }

    /// The bot answers in two bubbles, one poll apart: both come back, and
    /// what came before our message does not.
    #[test]
    fn a_reply_in_several_bubbles_comes_back_whole() {
        let polls = RefCell::new(0);
        let call = |request: &Request| {
            assert!(matches!(request, Request::ReadMessages { .. }));
            let mut messages = vec![message("old", false), message("ours", true)];
            let poll = {
                let mut polls = polls.borrow_mut();
                *polls += 1;
                *polls
            };
            if poll >= 2 {
                messages.push(message("first", false));
            }
            if poll >= 3 {
                messages.push(message("second", false));
            }
            Ok(json!({ "chat": "bot@s.whatsapp.net", "messages": messages }))
        };
        let answer = wait_for_reply(
            &call,
            "5511987654321",
            None,
            Duration::from_secs(5),
            Duration::from_millis(30),
            Duration::from_millis(5),
        )
        .unwrap();
        let ids: Vec<_> = answer["replies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|reply| reply["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["first", "second"]);
        assert_eq!(answer["chat"], "bot@s.whatsapp.net");
        assert_eq!(answer["timed_out"], false);
    }

    #[test]
    fn silence_times_out_with_no_replies() {
        let call = |_: &Request| Ok(json!({ "chat": "c", "messages": [message("ours", true)] }));
        let answer = wait_for_reply(
            &call,
            "c",
            None,
            Duration::from_millis(30),
            Duration::from_secs(3),
            Duration::from_millis(5),
        )
        .unwrap();
        assert_eq!(answer["timed_out"], true);
        assert_eq!(answer["replies"], json!([]));
    }

    #[test]
    fn replies_count_after_the_given_message() {
        let messages = [
            message("a", false),
            message("b", true),
            message("c", false),
            message("d", true),
            message("e", false),
        ];
        let ids = |anchor| {
            incoming_after(&messages, anchor)
                .iter()
                .map(|reply| reply["id"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Some("b")), ["c", "e"]);
        assert_eq!(ids(Some("d")), ["e"]);
        assert_eq!(ids(Some("gone")), ["a", "c", "e"]);
    }
}
