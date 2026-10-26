//! A single stdio MCP tool for the public fixture origin. No provider auth,
//! caller headers, redirects, proxies, arbitrary URLs or task-solving actions.
use crate::wrappers::Inputs;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Url;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    time::Duration,
};

pub const INPUT_LIMIT: usize = 64 * 1024;
pub const RESPONSE_LIMIT: usize = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    name: String,
    arguments: Arguments,
    // MCP request context is inert and never enters the HTTP arguments.
    #[serde(default)]
    _meta: serde::de::IgnoredAny,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arguments {
    method: String,
    path: String,
    #[serde(default, deserialize_with = "json_body")]
    body: Option<Value>,
}

fn json_body<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(d).map(Some)
}

fn request_url(base: &Url, path: &str) -> Result<Url, String> {
    // Decode once for admission as well as checking the URL library's final
    // origin. Refuse ambiguous separators/double encoding rather than depend
    // on a downstream router's decoding rules.
    let mut decoded = Vec::new();
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or("http_path_invalid")?;
            let hex = std::str::from_utf8(hex).map_err(|_| "http_path_invalid")?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| "http_path_invalid")?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = std::str::from_utf8(&decoded).map_err(|_| "http_path_invalid")?;
    if !decoded.starts_with('/')
        || decoded.starts_with("//")
        || decoded.contains(['\\', '#', '%'])
        || decoded.chars().any(char::is_control)
        || decoded.contains("://")
    {
        return Err("http_path_invalid".into());
    }
    let url = base.join(path).map_err(|_| "http_path_invalid")?;
    if url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("http_origin_invalid".into());
    }
    Ok(url)
}

pub struct Helper {
    inputs: Inputs,
    http: reqwest::Client,
    until: tokio::time::Instant,
}
impl Helper {
    pub fn new(inputs: Inputs) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| "http_client_unavailable")?;
        let until = tokio::time::Instant::now() + Duration::from_millis(inputs.deadline_ms);
        Ok(Self {
            inputs,
            http,
            until,
        })
    }
    pub async fn call(&self, arguments: Value) -> Result<Value, String> {
        if serde_json::to_vec(&arguments)
            .map_err(|_| "http_arguments_invalid")?
            .len()
            > INPUT_LIMIT
        {
            return Err("http_request_too_large".into());
        }
        let arguments: Arguments =
            serde_json::from_value(arguments).map_err(|_| "http_arguments_invalid")?;
        if !["GET", "POST"].contains(&arguments.method.as_str()) {
            return Err("http_method_invalid".into());
        }
        if arguments.method == "GET" && arguments.body.is_some() {
            return Err("http_get_body_not_supported".into());
        }
        let url = request_url(&self.inputs.app, &arguments.path)?;
        let remaining = self
            .until
            .saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("http_deadline_expired".into());
        }
        let until = tokio::time::Instant::now() + remaining.min(Duration::from_secs(30));
        let exchange = async {
            let mut request = self
                .http
                .request(
                    arguments
                        .method
                        .parse()
                        .map_err(|_| "http_method_invalid")?,
                    url,
                )
                .header("X-Eval-Identity", self.inputs.identity.clone());
            if let Some(body) = arguments.body {
                request = request
                    .header("content-type", "application/json")
                    .body(serde_json::to_vec(&body).map_err(|_| "http_body_invalid")?);
            }
            let mut response = request.send().await.map_err(|_| "http_request_failed")?;
            let status = response.status().as_u16();
            if response
                .content_length()
                .is_some_and(|n| n > RESPONSE_LIMIT as u64)
            {
                return Err("http_response_too_large".into());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| "http_response_failed")? {
                if chunk.len() > RESPONSE_LIMIT - bytes.len() {
                    return Err("http_response_too_large".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            // Preserve non-UTF-8 response bytes explicitly; never substitute a
            // lossy string or return a silently truncated successful response.
            Ok(match String::from_utf8(bytes) {
                Ok(body) => json!({"status":status,"body":body,"encoding":"utf-8"}),
                Err(error) => {
                    json!({"status":status,"body":STANDARD.encode(error.as_bytes()),"encoding":"base64"})
                }
            })
        };
        tokio::time::timeout_at(until, exchange)
            .await
            .map_err(|_| "http_request_timeout")?
    }
    pub async fn rpc(&self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned()?;
        if message.get("jsonrpc") != Some(&json!("2.0"))
            || !(id.is_null() || id.is_string() || id.is_i64() || id.is_u64())
        {
            return Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"invalid_request"}}),
            );
        }
        let result = match message.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                let version = message["params"]["protocolVersion"].as_str().unwrap_or("");
                if !["2025-03-26", "2025-06-18", "2025-11-25"].contains(&version) {
                    Err((-32602, "protocol_version_unsupported"))
                } else {
                    Ok(
                        json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"eval-fixture-http","version":"1"}}),
                    )
                }
            }
            Some("ping") => Ok(json!({})),
            Some("tools/list") => Ok(
                json!({"tools":[{"name":"eval_http","description":"Request the public fixture app. Identity is supplied automatically, including GET /. Read GET / to discover the app's forms. Paths are app-relative; mail is read with Swarmail MCP.","inputSchema":{"type":"object","properties":{"method":{"type":"string","enum":["GET","POST"]},"path":{"type":"string"},"body":{}},"required":["method","path"],"additionalProperties":false}}]}),
            ),
            Some("tools/call") => match serde_json::from_value::<Call>(message["params"].clone()) {
                Ok(call) if call.name == "eval_http" => {
                    let arguments =
                        json!({"method":call.arguments.method,"path":call.arguments.path});
                    let mut arguments = arguments;
                    if let Some(body) = call.arguments.body {
                        arguments["body"] = body;
                    }
                    match self.call(arguments).await {
                        Ok(value) => Ok(
                            json!({"content":[{"type":"text","text":value.to_string()}],"isError":false}),
                        ),
                        Err(error) => {
                            let message = match error.as_str() {
                                "http_get_body_not_supported" => "http_get_body_not_supported",
                                _ => "fixture_request_refused",
                            };
                            Ok(json!({"content":[{"type":"text","text":message}],"isError":true}))
                        }
                    }
                }
                _ => Err((-32602, "tool_arguments_invalid")),
            },
            _ => Err((-32601, "method_not_found")),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err((code, message)) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
            }
        })
    }
}

fn line(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut bytes = Vec::new();
    loop {
        let buf = reader.fill_buf().map_err(|_| "helper_input_failed")?;
        if buf.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err("helper_input_truncated".into())
            };
        }
        let end = buf.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let count = end.unwrap_or(buf.len());
        if count > INPUT_LIMIT - bytes.len() {
            return Err("helper_input_too_large".into());
        }
        bytes.extend_from_slice(&buf[..count]);
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(bytes));
        }
    }
}

pub async fn serve(inputs: Inputs) -> Result<(), String> {
    let helper = Helper::new(inputs)?;
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    while let Some(bytes) = line(&mut reader)? {
        let message: Value = serde_json::from_slice(&bytes).map_err(|_| "helper_json_invalid")?;
        if let Some(reply) = helper.rpc(message).await {
            serde_json::to_writer(&mut out, &reply).map_err(|_| "helper_output_failed")?;
            out.write_all(b"\n")
                .and_then(|_| out.flush())
                .map_err(|_| "helper_output_failed")?;
        }
    }
    Ok(())
}
