use async_process::{Command, Stdio};
// type of BufReader::new(stderr).lines()
use futures_lite::AsyncBufReadExt;
use futures_lite::io::BufReader;
// type of `tasks.next()
use futures_lite::stream::StreamExt;
use futures_util::stream::FuturesUnordered;
// type of `task.timeout()` in match
use smol_timeout::TimeoutExt;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use brace_expand::brace_expand;
use shellexpand;

mod logging;

mod environment;
pub use environment::read_env_variables;

// @TODO print Stderr on non-zero exit, maybe we just go the log_err route
//       but it would be nicer to actually return an error and print it
//
// @TODO can we poll_once for both stderr and stdout after waiting for the task?
//
// @TODO cleanup

struct Args {
    show_header: bool,
    use_color: bool,
    in_repos: bool, // whether to operate on files or in repos
    config_filename: String,
    timeout: Option<Duration>,
    command: Vec<String>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut show_header = true;
    let mut use_color = true;
    let mut in_repos = true;
    let mut timeout: Option<Duration> = None;
    let mut config_filename: String = "repo.conf".to_string();
    let mut command: Vec<String> = Vec::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("timeout") => {
                timeout = Some(Duration::from_secs(parser.value()?.parse()?));
            }

            Long("no-header") => {
                show_header = false;
            }

            Long("no-color") => {
                use_color = false;
            }

            Short('f') | Long("files") => {
                in_repos = false;
            }

            Short('c') | Long("config") => {
                if let Ok(value) = parser.value() {
                    let value_str = value.to_string_lossy();
                    config_filename = value_str.to_string();
                }
            }

            Value(val) => {
                command.push(val.string()?);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    log_info!("config file: {:}", config_filename);
    Ok(Args {
        show_header: show_header,
        use_color: use_color,
        in_repos: in_repos,
        config_filename,
        timeout: timeout,
        command: if command.is_empty() {
            return Err("missing command/args".into());
        } else {
            command
        },
    })
}

// fn collect_lines_non_blocking() {

// }

async fn collect_lines_async<T: AsyncBufReadExt + Unpin>(
    lines: &mut futures_lite::io::Lines<T>,
) -> String {
    let mut out = Vec::new();
    while let Some(line) = lines.next().await {
        if let Ok(line) = line {
            out.push(line);
        }
    }
    out.join("\n")
}

async fn run_command(
    cmd: String,
    arguments: Vec<String>,
    file: PathBuf,
    show_header: bool,
    use_color: bool,
    is_repos: bool,
    timeout: Option<Duration>,
) -> Result<(String, String), String> {
    let mut args = arguments.clone();

    if cmd == "git" && use_color {
        args.insert(0, "-c".to_string());
        args.insert(1, "color.status=always".to_string());
    }

    let mut command = Command::new(cmd);
    if is_repos {
        command.current_dir(file.clone());
    } else {
        args.push(file.to_string_lossy().to_string());
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command
        .args(args)
        .spawn()
        .map_err(|e| format!("spawn failed: {}", e))?;

    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
    let task = async {
        let status = child
            .status()
            .await
            .map_err(|e| format!("wait failed: {}", e))?;
        let fname = file.to_string_lossy().into_owned();

        if !status.success() {
            debug!("Non-zero exit. file: {:?}", file);
            // Err("Non-zero");
        }
        let out_str = collect_lines_async(&mut stdout).await;

        Ok((fname, out_str))
    };

    if let Some(to) = timeout {
        if let Some(res) = task.timeout(to).await {
            res
        } else {
            // Kill the process (best effort)
            let _ = child.kill();
            let dir = file.to_string_lossy().into_owned();

            let mut stderr_buf: Vec<String> = Vec::new();
            if let Some(stderr) = child.stderr.take() {
                let mut lines = BufReader::new(stderr).lines();
                loop {
                    // poll_once returns a future; you must await it to get Option<Option<String>>
                    match smol::future::poll_once(lines.next()).await {
                        Some(Some(line)) => stderr_buf.push(line.unwrap()), // got a line
                        Some(None) => break,                                // stream ended
                        None => break, // nothing available right now; exit quickly
                    }
                }
            } else {
                // child.stderr was already taken, handle as error
                debug!("Cannot read stderr, already taken. file: {:?}", file);
                // Err("Cannot read stderr, already taken.");
            }

            debug!("stderr_buf: {:?}", stderr_buf);
            if stderr_buf.is_empty() {
                Err(format!("timed out in '{}' after {:?}.", dir, to))
            } else {
                Err(format!(
                    "timed out in '{}' after {:?}. stderr: {:?}",
                    dir,
                    to,
                    stderr_buf.join("\n")
                ))
            }
        }
    } else {
        // this condition does not use a timeout
        task.await
    }
}

fn get_files(config_filename: String, home: String) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();

    let mut config_path = PathBuf::from(config_filename.as_str());
    if !config_path.is_absolute() {
        debug!("config_path: {:?} is not absolute.", config_path);
        config_path =
            PathBuf::from(format!("{}/{}/{}", home, ".config/personal", config_filename).as_str());
        debug!("updated config_path: {:?}", config_path);
    }

    let config = fs::read_to_string(config_path);
    let lines: Vec<String> = config
        .expect("new lines missing")
        .lines()
        .map(|line| line.to_string())
        .collect();

    // debug!("lines: {:?}", lines);
    for line in lines {
        // debug!("line: {:?}", line);
        if line.starts_with("#") || line.len() < 1 {
            continue;
        }

        let brace_expanded = brace_expand(line.as_str());
        let fully_expanded: Vec<String> = brace_expanded
            .iter()
            .map(|item| {
                shellexpand::full(&item)
                    .expect("expansion failed")
                    .into_owned()
            })
            .collect();
        // debug!("fully_expanded: {:?}", fully_expanded);
        // debug!("{}", "---");

        for file in fully_expanded {
            files.push(file);
        }
    }

    debug!("files: {:?}", files);
    return files;
}

fn main() -> Result<(), lexopt::Error> {
    let env_keys = ["HOME"];
    let env = read_env_variables(&env_keys);
    let home = env["HOME"].clone();

    let args = parse_args()?;
    let show_header = args.show_header;
    let use_color = args.use_color;
    let in_repos = args.in_repos;
    let timeout = args.timeout;
    // let timeout: Option<Duration> = Some(Duration::from_secs(1));
    // let timeout: Option<Duration> = None;
    let command = args.command;
    let config_filename = args.config_filename;

    let files = get_files(config_filename, home);
    // let _ = get_files(config_filename, home);
    // let files = vec![PathBuf::from("/Users/florian.sorko/Repos/scripts")];

    let cmd = command[0].clone();
    let mut cmd_args: Vec<String> = Vec::new();

    for arg in command.clone().split_off(1) {
        cmd_args.push(arg);
    }

    smol::block_on(async {
        let mut tasks = FuturesUnordered::new();

        for file in files {
            tasks.push(smol::spawn(run_command(
                cmd.clone(),
                cmd_args.clone(),
                file.clone().into(),
                show_header,
                use_color,
                in_repos,
                timeout,
            )));
        }

        while let Some(result) = tasks.next().await {
            if let Ok((file, stdout)) = result {
                if in_repos {
                    if stdout.len() < 1 {
                        println!("--\nFinished: '{}'", file);
                    } else {
                        println!("--\nFinished: '{}'\n{}", file, stdout);
                    }
                } else {
                    let mut header = format!("--\nFinished:'{}'\n", file.as_str());
                    if !show_header {
                        header = "".to_string();
                    }
                    if stdout.len() < 1 {
                        print!("{}", header);
                    } else {
                        println!("{}{}", header, stdout);
                    }
                }
            } else if let Err(err) = result {
                eprintln!("--\nError: {}", err);
            } else {
                eprintln!("--\nWe should never reach this");
            }
        }
    });

    Ok(())
}
