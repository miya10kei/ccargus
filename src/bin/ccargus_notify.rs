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

    let msg = serde_json::json!({ "cwd": cwd, "status": status });
    let payload = msg.to_string();

    for path in discover_sockets() {
        if let Ok(mut stream) = UnixStream::connect(&path) {
            let _ = std::io::Write::write_all(&mut stream, payload.as_bytes());
        }
    }
}

fn discover_sockets() -> Vec<PathBuf> {
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
    let dir = runtime_dir.join("ccargus");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name().to_str().is_some_and(|n| {
                n.starts_with("notify-")
                    && std::path::Path::new(n)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
            })
        })
        .map(|e| e.path())
        .collect()
}
