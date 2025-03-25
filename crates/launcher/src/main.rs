use shared::AppConfig;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::{env, thread};

fn main() -> anyhow::Result<()> {
    let config = AppConfig::new();

    let exe_path = env::current_exe()?.parent().unwrap().to_path_buf();
    let backend_dir = match config.zenzai.backend.as_str() {
        "cpu" => "llama_cpu",
        "cuda" => "llama_cuda",
        "vulkan" => "llama_vulkan",
        _ => "llama_cpu",
    };

    let backend_path = exe_path.join(backend_dir);
    let backend_path_str = backend_path.to_string_lossy();

    let mut new_path = env::var("PATH").unwrap_or_else(|_| String::new());
    new_path = format!("{};{}", backend_path_str, new_path);
    env::set_var("PATH", &new_path);

    let server_process = start_process("azookey-server.exe", "[server]");
    let ui_process = start_process("ui.exe", "[ui]");

    if let (Some(mut server), Some(mut ui)) = (server_process, ui_process) {
        let server_handle = thread::spawn(move || server.wait());
        let ui_handle = thread::spawn(move || ui.wait());

        let _ = server_handle.join();
        let _ = ui_handle.join();
    }

    Ok(())
}

fn start_process(exe: &str, prefix: &str) -> Option<Child> {
    let mut child = Command::new(exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(&format!("Failed to start {}", exe));

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stdout_reader = BufReader::new(stdout);
    let prefix_stdout = prefix.to_string();
    thread::spawn(move || {
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                println!("{}: {}", prefix_stdout, line);
            }
        }
    });

    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let stderr_reader = BufReader::new(stderr);
    let prefix_stderr = prefix.to_string();
    thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                eprintln!("{}: {}", prefix_stderr, line);
            }
        }
    });

    Some(child)
}
