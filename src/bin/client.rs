//! Клиент для FIFO-based task manager.

use clap::Parser;
use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Базовый путь к именованным каналам (без расширения)
    #[arg(long, default_value = "./pipe")]
    pipe_name: String,

    /// Тип запроса: add, get, delete
    #[arg(long)]
    request: String,

    /// Параметр запроса: название задачи (для add) или ID (для get/delete)
    #[arg(long)]
    parameter: Option<String>,
}

#[derive(Debug, Serialize)]
struct Request {
    #[serde(rename = "type")]
    req_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<u64>,
    response_pipe: String,
}

#[derive(Debug, Deserialize)]
struct Response {
    response_status: String,
    task_id: Option<u64>,
    task_name: Option<String>,
    message: Option<String>,
}

fn cleanup_response_pipe(response_pipe: &str) {
    let _ = fs::remove_file(response_pipe);
}

fn main() {
    let args = Args::parse();

    let pid = process::id();
    let response_pipe = format!("{}.response.{}", args.pipe_name, pid);
    let request_pipe = format!("{}.request", args.pipe_name);
    if let Err(e) = mkfifo(response_pipe.as_str(), Mode::S_IRUSR | Mode::S_IWUSR) {
        eprintln!("Ошибка создания response pipe: {}", e);
        return;
    }

    let request = match args.request.as_str() {
        "add" => Request {
            req_type: "add".to_string(),
            task_name: args.parameter,
            task_id: None,
            response_pipe: response_pipe.clone(),
        },
        "get" | "delete" => {
            let id = match args.parameter.as_ref() {
                Some(p) => match p.parse::<u64>() {
                    Ok(i) => i,
                    Err(_) => {
                        eprintln!("Ошибка: ID должен быть числом");
                        cleanup_response_pipe(&response_pipe);
                        return;
                    }
                },
                None => {
                    eprintln!("Ошибка: параметр обязателен для get/delete");
                    cleanup_response_pipe(&response_pipe);
                    return;
                }
            };
            Request {
                req_type: args.request,
                task_name: None,
                task_id: Some(id),
                response_pipe: response_pipe.clone(),
            }
        }
        _ => {
            eprintln!("Неизвестная команда: {}", args.request);
            cleanup_response_pipe(&response_pipe);
            return;
        }
    };

    let mut req_stream = match fs::OpenOptions::new().write(true).open(&request_pipe) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Не удалось открыть request pipe: {}", e);
            eprintln!("  Убедитесь, что сервер запущен");
            cleanup_response_pipe(&response_pipe);
            return;
        }
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize request");
    if let Err(e) = req_stream.write_all(json.as_bytes()) {
        eprintln!("Ошибка отправки: {}", e);
        cleanup_response_pipe(&response_pipe);
        return;
    }
    drop(req_stream);

    let mut resp_stream = match fs::OpenOptions::new().read(true).open(&response_pipe) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Не удалось открыть response pipe: {}", e);
            cleanup_response_pipe(&response_pipe);
            return;
        }
    };

    let mut buffer = String::new();
    if let Err(e) = resp_stream.read_to_string(&mut buffer) {
        eprintln!("Ошибка чтения ответа: {}", e);
        cleanup_response_pipe(&response_pipe);
        return;
    }

    // Очистка временного FIFO
    cleanup_response_pipe(&response_pipe);

    match serde_json::from_str::<Response>(&buffer) {
        Ok(resp) => {
            println!("{}", buffer);
            eprintln!("\nРезультат:");
            eprintln!("  Статус: {}", resp.response_status);
            if let Some(id) = resp.task_id {
                eprintln!("  ID: {}", id);
            }
            if let Some(name) = resp.task_name {
                eprintln!("  Задача: {}", name);
            }
            if let Some(msg) = resp.message {
                eprintln!("  Сообщение: {}", msg);
            }
        }
        Err(e) => {
            eprintln!("Ошибка парсинга JSON: {}", e);
            eprintln!("  Сырой ответ: {}", buffer);
        }
    }
}
