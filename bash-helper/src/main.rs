use gethostname::gethostname;

use std::fs;
use std::process::Command;

mod logging;

mod environment;
pub use environment::read_env_variables;

fn refresh_tmux() {
    let _ = Command::new("tmux")
        .arg("refresh-client")
        .spawn()
        .map_err(|e| {
            debug!("Failed to execute `tmux refresh-client`: {}", e);
        });
}

fn update_tmux_display(os_cloud: &str, kubecfg: &str) {
    fs::write("/tmp/._kubeconfig", kubecfg).unwrap();
    fs::write("/tmp/._openstack_cloud", os_cloud).unwrap();

    refresh_tmux();
}

fn print_shortened_path(
    path: &str,
    home: &str,
    color: &str,
    not_host_env_color: &str,
    no_color: &str,
    in_container: bool,
) {
    let mut prefix: String = "".to_string();
    if in_container {
        prefix += not_host_env_color;
        prefix += format!("{}{}", gethostname().display().to_string(), ": ").as_str();
        prefix += no_color;
    }

    // let mut tokens: Vec<&str> = path.split('/').collect();
    let mut tokens: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
    debug!("tokens before drain: {:?}", tokens);
    if home.len() > 0 && path.starts_with(home) {
        prefix += "~/";

        if path.len() == home.len() {
            tokens.clear();
        } else {
            if tokens.len() > 3 {
                tokens.drain(0..3);
            }
        }
    }
    debug!("prefix: {}", prefix);
    debug!("tokens after drain: {:?}", tokens);

    // if we do not cast this will end up an uint
    // this will result in a crash if a substractions
    // results in a value less than 0.
    let tokens_len = tokens.len() as i8;
    debug!("tokens_len: {}", tokens_len);
    let mut tokens_idx = -1;
    for t in &mut tokens {
        tokens_idx += 1;
        debug!("token: {}, tokens_idx: {}", t, tokens_idx);

        if tokens_idx == (tokens_len - 2) || tokens_idx == (tokens_len - 3) {
            continue;
        }
        if tokens_idx == (tokens_len - 1) {
            break;
        }

        if t.len() > 1 {
            if t.starts_with(".") {
                t.truncate(2);
            } else {
                t.truncate(1);
            }
        }
    }
    debug!("tokens after truncation: {:?}", tokens);
    let joined = tokens.join("/");
    debug!("joined: {}", joined);
    print!("{}{}{}{}", color, prefix, joined, no_color);
}

fn main() {
    let env_keys = [
        "HOME",
        "PWD",
        "NOT_HOST_ENV",
        "OS_CLOUD",
        "KUBECONFIG",
        "GREEN",
        "BLUE",
        "RED",
        "NC",
        "VIRTUAL_ENV",
    ];
    let env = read_env_variables(&env_keys);
    let pwd = env["PWD"].as_str();
    let venv = env["VIRTUAL_ENV"].as_str();
    let home = env["HOME"].as_str();
    let blue = env["BLUE"].as_str();
    let green = env["GREEN"].as_str();
    let red = env["RED"].as_str();
    let no_color = env["NC"].as_str();

    debug!("venv: {}", venv);

    let not_host_env = env["NOT_HOST_ENV"].as_str();
    let in_container = if not_host_env.len() == 0 { false } else { true };

    print_shortened_path(pwd, home, green, red, no_color, in_container);
    if venv.len() > 0 {
        print!(" (");
        print_shortened_path(venv, home, blue, red, no_color, in_container);
        print!(")");
    }
    println!("\n$ ");

    update_tmux_display(env["OS_CLOUD"].as_str(), env["KUBECONFIG"].as_str());
}
