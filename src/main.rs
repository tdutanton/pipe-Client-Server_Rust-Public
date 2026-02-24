//! Сервер с именованными каналами (FIFO).

use clap::Parser;
use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "./pipe")]
    pipe_name: String,
}

#[derive(Debug, Deserialize)]
struct Request {
    #[serde(rename = "type")]
    req_type: String,
    task_name: Option<String>,
    task_id: Option<u64>,
    response_pipe: String,
}

#[derive(Debug, Serialize)]
struct Response {
    response_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct Task {
    id: u64,
    name: String,
}

struct ServerState {
    tasks: HashMap<u64, Task>,
    next_id: u64,
}

impl ServerState {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
        }
    }
}

fn handle_request(request: Request, state: &mut ServerState) -> Response {
    match request.req_type.as_str() {
        "add" => {
            if state.tasks.len() >= 5 {
                Response {
                    response_status: "fail".to_string(),
                    task_id: None,
                    task_name: None,
                    message: Some("too many tasks".to_string()),
                }
            } else {
                let task_name = request.task_name.unwrap_or_default();
                let new_id = state.next_id;
                state.next_id += 1;
                state.tasks.insert(
                    new_id,
                    Task {
                        id: new_id,
                        name: task_name.clone(),
                    },
                );
                Response {
                    response_status: "success".to_string(),
                    task_id: Some(new_id),
                    task_name: None,
                    message: None,
                }
            }
        }
        "get" => {
            let id = request.task_id.unwrap_or(0);
            state
                .tasks
                .get(&id)
                .map(|task| Response {
                    response_status: "success".to_string(),
                    task_id: Some(task.id),
                    task_name: Some(task.name.clone()),
                    message: None,
                })
                .unwrap_or_else(|| Response {
                    response_status: "fail".to_string(),
                    task_id: None,
                    task_name: None,
                    message: Some("task not found".to_string()),
                })
        }
        "delete" => {
            let id = request.task_id.unwrap_or(0);
            state
                .tasks
                .remove(&id)
                .map(|task| Response {
                    response_status: "success".to_string(),
                    task_id: Some(task.id),
                    task_name: Some(task.name),
                    message: None,
                })
                .unwrap_or_else(|| Response {
                    response_status: "fail".to_string(),
                    task_id: None,
                    task_name: None,
                    message: Some("task not found".to_string()),
                })
        }
        _ => Response {
            response_status: "fail".to_string(),
            task_id: None,
            task_name: None,
            message: Some("unknown request type".to_string()),
        },
    }
}

fn send_response(response_pipe: &str, response: &Response) -> std::io::Result<()> {
    let json = serde_json::to_string(response)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut stream = fs::OpenOptions::new().write(true).open(response_pipe)?;
    stream.write_all(json.as_bytes())?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let request_pipe = format!("{}.request", args.pipe_name);

    let _ = fs::remove_file(&request_pipe);
    mkfifo(request_pipe.as_str(), Mode::S_IRUSR | Mode::S_IWUSR).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("mkfifo failed: {}", e)))?;

    println!(
        "Сервер запущен. Ожидает запросы на: {}.request",
        args.pipe_name
    );

    let mut state = ServerState::new();

    loop {
        let mut req_stream = match fs::OpenOptions::new().read(true).open(&request_pipe) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Ошибка открытия request pipe: {}", e);
                continue;
            }
        };

        let mut buffer = String::new();
        if let Err(e) = req_stream.read_to_string(&mut buffer) {
            eprintln!("Ошибка чтения: {}", e);
            continue;
        }

        if buffer.trim().is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(&buffer) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Неверный JSON: {}", e);
                continue;
            }
        };

        let response_pipe = request.response_pipe.clone();
        let response = handle_request(request, &mut state);

        if let Err(e) = send_response(&response_pipe, &response) {
            eprintln!("Не удалось отправить ответ: {}", e);
        }
    }
}
