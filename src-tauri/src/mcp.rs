//! Minimal MCP server over stdio (newline-delimited JSON-RPC 2.0).
//! Started with `port-ontology.exe --mcp`. Logs go to stderr only; stdout is
//! reserved for protocol messages.

use crate::engine::Engine;
use crate::tools;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

const SUPPORTED_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub fn run() {
    let mut engine = Engine::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    eprintln!("port-ontology MCP server ready ({} containers loaded)", engine.o.containers.len());

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&mut engine, &msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("Parse error: {e}"))),
        };
        if let Some(r) = response {
            if writeln!(stdout, "{r}").and_then(|_| stdout.flush()).is_err() {
                break;
            }
        }
    }
}

fn result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn handle(engine: &mut Engine, msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    // Notifications (no id) never get a response.
    let id = msg.get("id").cloned()?;
    let params = msg.get("params").cloned().unwrap_or(json!({}));

    Some(match method {
        "initialize" => {
            let requested = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
            let version = if SUPPORTED_VERSIONS.contains(&requested) { requested } else { SUPPORTED_VERSIONS[0] };
            result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "port-ontology", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": "컨테이너 터미널 온톨로지 서버입니다. 먼저 describe_schema로 모델을 파악하세요. '지연되면 어떻게 되나' 같은 질문에는 trace_delay_impact를 쓰고 답변에 evidence_path를 근거로 인용하세요. propose_* 도구는 검증만 하며, apply_plan은 사용자가 확인한 뒤에만 호출하세요. 사용자에게는 한국어로 답하세요."
                }),
            )
        }
        "ping" => result(id, json!({})),
        "tools/list" => result(id, json!({"tools": tools::definitions()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match tools::dispatch(engine, name, &args) {
                Ok(v) => result(
                    id,
                    json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap()}], "isError": false}),
                ),
                // Tool failures are reported in-band so the model can recover.
                Err(e) => result(id, json!({"content": [{"type": "text", "text": e}], "isError": true})),
            }
        }
        _ => error(id, -32601, &format!("Method not found: {method}")),
    })
}
