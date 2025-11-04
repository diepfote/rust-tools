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
use globby::glob;
use shellexpand::full;

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
            Short('t') | Long("timeout") => {
                timeout = Some(Duration::from_secs(parser.value()?.parse()?));
            }

            Short('w') | Long("max-concurrent-tasks") => {
                max_concurrent_tasks = parser.value()?.parse()?;
            }

            Long("no-header") => {
                show_header = false;
            }

            Long("no-color") => {
                use_color = false;
            }

            Long("files") => {
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

    if in_repos && timeout == None {
        timeout = Some(Duration::from_secs(3));
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
    in_repos: bool,
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

    let mut command = Command::new(cmd.clone());
    if in_repos {
        command.current_dir(file.clone());
    } else {
        args.push(file.to_string_lossy().to_string());
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut err_info = format!(
        "Spawn failed in '{}'. Cmd: {:?}, Args: {:?}",
        file.to_string_lossy().into_owned(),
        cmd,
        args.clone(),
    );
    if !in_repos {
        err_info = format!(
            "Spawn failed (--files): Cmd: {:?}, Args: {:?}",
            cmd,
            args.clone(),
        );
    }
    let mut child = command
        .args(args)
        .spawn()
        .map_err(|e| format!("{}: {}", err_info, e))?;

    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();

    let task = async {
        let status = child
            .status()
            .await
            .map_err(|e| format!("Wait failed for '{:?}': {}", file, e))?;
        let fname = file.to_string_lossy().into_owned();

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

            let mut stderr_display = "".to_string();
            if stderr_str.len() > 0 {
                stderr_display = format!("\n[.] stderr:\n{}", stderr_str);
            }
            Err(format!(
                "Timed out in '{}' after {:?}.\n{}{}",
                dir, to, stdout_str, stderr_display
            ))
        }
    } else {
        // this condition does not use a timeout
        task.await
    }
}

fn get_paths(config_filename: String, home: String) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();

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
        if line.starts_with("#") || line.trim().len() < 1 {
            continue;
        }

        let shell_expanded: String = full(&line).expect("shellexpand failed").into_owned();
        debug!("shell_expanded: {}", shell_expanded);

        if !shell_expanded.contains("*") && !shell_expanded.contains("{") {
            paths.push(shell_expanded.clone());
            continue;
        }

        let brace_expanded = brace_expand(&shell_expanded);
        debug!("brace_expanded: {:?}", brace_expanded);

        let mut glob_expanded: Vec<String> = Vec::new();
        for expanded in brace_expanded {
            let mut globbed: Vec<String> = glob(&expanded)
                .expect("Glob failed")
                .map(|item| item.expect("Error on path in glob").display().to_string())
                .collect();

            glob_expanded.append(&mut globbed);
        }
        debug!("glob_expanded: {:?}", glob_expanded);

        for path in glob_expanded {
            paths.push(path);
        }
    }
    debug!("paths: {:?}", paths);
    paths
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
    log_info!("number of concurrent tasks: {}", max_concurrent_tasks);

    let paths = get_paths(config_filename, home);

    let mut name = "files".to_string();
    if in_repos {
        name = "repos".to_string();
    }
    let number_of_paths = paths.len();
    log_info!("number of {}: {}", name, number_of_paths);
    if let Some(timeout) = timeout {
        log_info!("timeout: {:?}", timeout);
    }

    let cmd = command[0].clone();
    let mut cmd_args: Vec<String> = Vec::new();

    for arg in command.clone().split_off(1) {
        cmd_args.push(arg);
    }

    smol::block_on(async {
        let mut tasks = FuturesUnordered::new();
        let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));

        for file in paths {
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

        let mut tasks_done = 0;
        while let Some(result) = tasks.next().await {
            if let Ok((file, exit_code, stdout, stderr)) = result {
                let mut exit_info = "".to_string();
                let ec = exit_code.unwrap();
                if ec != 0 {
                    exit_info = format!("[-] Non-zero {}: ", ec.to_string());
                }
                let mut header = format!("--\n{}'{}'\n", exit_info, file.as_str());
                if !show_header {
                    header = "".to_string();
                }

                let mut stderr_display = "".to_string();
                if stderr.len() > 0 {
                    stderr_display = format!("\n[.] stderr:\n{}", stderr);
                }
                println!("{}{}{}", header, stdout, stderr_display);
            } else if let Err(err) = result {
                eprintln!("--\n! {}", err);
            }

            if !show_header {
                tasks_done += 1;
                if tasks_done % 10 == 0 {
                    let mut remaining_tasks = 0;
                    if number_of_paths > 0 {
                        remaining_tasks = number_of_paths - tasks_done;
                    }
                    log_info!("remaining tasks: {}", remaining_tasks);
                }
            }
        }
    });
    Ok(())
}
