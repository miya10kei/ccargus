use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process;

fn main() {
    let Some(status) = std::env::args().nth(1) else {
        eprintln!("Usage: ccargus-notify <processing|waiting_for_input>");
        process::exit(1);
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        process::exit(0);
    }

    let cwd = serde_json::from_str::<serde_json::Value>(&input)
        .ok()
        .and_then(|v| v.get("cwd").and_then(|c| c.as_str()).map(String::from))
        .unwrap_or_default();

    if cwd.is_empty() {
        process::exit(0);
    }

    let socket_path = socket_path();
    let Ok(mut stream) = UnixStream::connect(&socket_path) else {
        process::exit(0);
    };

    let msg = serde_json::json!({ "cwd": cwd, "status": status });
    let _ = std::io::Write::write_all(&mut stream, msg.to_string().as_bytes());
}

fn socket_path() -> PathBuf {
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
    runtime_dir.join("ccargus/notify.sock")
}
