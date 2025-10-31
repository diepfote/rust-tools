use async_process::{Command, Stdio};

// type of BufReader::new(stderr).lines()
use futures_lite::AsyncBufReadExt;
use futures_lite::io::BufReader;
// type of `tasks.next()
use futures_lite::stream::StreamExt;
use futures_util::stream::FuturesUnordered;

// type of `task.timeout()` in match
use smol::lock::Semaphore;
// to ensure the Semaphore is clonable
use smol_timeout::TimeoutExt;
use std::sync::Arc;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use brace_expand::brace_expand;
use shellexpand;

mod logging;

mod environment;
pub use environment::read_env_variables;

struct Args {
    show_header: bool,
    use_color: bool,
    in_repos: bool, // whether to operate on files or in repos
    config_filename: String,
    max_concurrent_tasks: usize,
    timeout: Option<Duration>,
    command: Vec<String>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut show_header = true;
    let mut use_color = true;
    let mut in_repos = true;
    let mut timeout: Option<Duration> = None;
    let mut max_concurrent_tasks = 4;
    let mut config_filename: String = "repo.conf".to_string();
    let mut command: Vec<String> = Vec::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("timeout") => {
                timeout = Some(Duration::from_secs(parser.value()?.parse()?));
            }

            Short('t') | Long("max-concurrent-tasks") => {
                max_concurrent_tasks = parser.value()?.parse()?;
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

    Ok(Args {
        show_header,
        use_color,
        in_repos,
        config_filename,
        max_concurrent_tasks,
        timeout,
        command: if command.is_empty() {
            return Err("missing command/args".into());
        } else {
            command
        },
    })
}

async fn collect_lines_poll_once<T: futures_lite::AsyncBufRead + Unpin>(
    lines: &mut futures_lite::io::Lines<T>,
) -> String {
    let mut buf = Vec::new();
    loop {
        match smol::future::poll_once(lines.next()).await {
            Some(Some(Ok(line))) => buf.push(line),
            Some(Some(Err(err))) => {
                debug!("Error reading line: {:?}", err);
                break;
            }
            Some(None) | None => break, // Stream ended or nothing available
        }
    }
    buf.join("\n")
}

async fn run_command(
    cmd: String,
    arguments: Vec<String>,
    file: PathBuf,
    use_color: bool,
    is_repos: bool,
    timeout: Option<Duration>,
) -> Result<(String, Option<i32>, String, String), String> {
    let mut args = arguments.clone();

    let mut idx: usize = 0;
    if use_color {
        if cmd == "git" {
            args.insert(idx, "-c".to_string());
            idx += 1;
            args.insert(idx, "color.status=always".to_string());
            idx += 1;
        } else if cmd == "grep" {
            args.insert(idx, "--color=always".to_string());
            idx += 1;
        }
    }
    if cmd == "grep" {
        args.insert(idx, "--exclude-dir=.git".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.helm".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.tox".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.pulumi".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.cache".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.mypy_cache".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.eggs".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=*.egg-info".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=*venv*".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=_build".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=__pycache__".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.ruff_cache".to_string());
        idx += 1;
        args.insert(idx, "--exclude=\"*.pyc\"".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=.pytest_cache".to_string());
        idx += 1;
        args.insert(idx, "--exclude=poetry.lock".to_string());
        idx += 1;
        args.insert(idx, "--exclude-dir=htmlcov".to_string());
        idx += 1;
        args.insert(idx, "--exclude=\"*.html\"".to_string());
        idx += 1;
        args.insert(idx, "--exclude=build.*trace".to_string());
        idx += 1;
        args.insert(idx, "--exclude=Session.vim".to_string());
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
    let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

    let task = async {
        let status = child
            .status()
            .await
            .map_err(|e| format!("wait failed: {}", e))?;
        let fname = file.to_string_lossy().into_owned();

        // if !status.success() {
        //     debug!("Non-zero exit. file: {:?}", file);
        // }
        let stdout_str = collect_lines_poll_once(&mut stdout).await;
        let stderr_str = collect_lines_poll_once(&mut stderr).await;

        Ok((fname, status.code(), stdout_str, stderr_str))
    };

    if let Some(to) = timeout {
        if let Some(res) = task.timeout(to).await {
            res
        } else {
            // Kill the process (best effort)
            let _ = child.kill();
            let dir = file.to_string_lossy().into_owned();

            let stdout_str = collect_lines_poll_once(&mut stdout).await;
            let stderr_str = collect_lines_poll_once(&mut stderr).await;

            Err(format!(
                "timed out in '{}' after {:?}.\nstdout:\n{:}\nstderr:\n{:}",
                dir, to, stdout_str, stderr_str
            ))
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

    for line in lines {
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

        for file in fully_expanded {
            files.push(file);
        }
    }

    debug!("files: {:?}", files);
    files
}

fn main() -> Result<(), lexopt::Error> {
    let env_keys = ["HOME"];
    let env = read_env_variables(&env_keys);
    let home = env["HOME"].clone();

    let args = parse_args()?;
    let show_header = args.show_header;
    let use_color = args.use_color;
    let in_repos = args.in_repos;
    let max_concurrent_tasks = args.max_concurrent_tasks;
    let timeout = args.timeout;
    let command = args.command;
    let config_filename = args.config_filename;

    log_info!("config file: {:}", config_filename);
    log_info!("number of tasks: {}", max_concurrent_tasks);

    let files = get_files(config_filename, home);

    let mut name = "files".to_string();
    if in_repos {
        name = "repos".to_string();
    }
    log_info!("number of {}: {}", name, files.len());

    let cmd = command[0].clone();
    let mut cmd_args: Vec<String> = Vec::new();

    for arg in command.clone().split_off(1) {
        cmd_args.push(arg);
    }

    smol::block_on(async {
        let mut tasks = FuturesUnordered::new();
        let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));

        for file in files {
            let sem_clone = semaphore.clone();
            let cmd_clone = cmd.clone();
            let cmd_args_clone = cmd_args.clone();

            tasks.push(smol::spawn(async move {
                // will be release when it goes out of scope
                let _permit = sem_clone.acquire().await;

                let result = run_command(
                    cmd_clone,
                    cmd_args_clone,
                    file.clone().into(),
                    use_color,
                    in_repos,
                    timeout,
                )
                .await;

                result
            }));
        }

        while let Some(result) = tasks.next().await {
            if let Ok((file, exit_code, stdout, stderr)) = result {
                let mut header = format!(
                    "--\nExit {}: '{}'\n",
                    exit_code.unwrap().to_string(),
                    file.as_str()
                );
                if !show_header {
                    header = "".to_string();
                }

                if stderr.len() < 1 {
                    println!("{}{}", header, stdout);
                } else {
                    println!("{}stdout:\n{}\nstderr:\n{}", header, stdout, stderr);
                }
            } else if let Err(err) = result {
                // we hit this in case of a timeout
                eprintln!("--\nError: {}", err);
            } else {
                eprintln!("--\nWe should never reach this");
            }
        }
    });
    Ok(())
}
