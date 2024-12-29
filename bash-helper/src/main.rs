use std::collections::HashMap;
use std::env;

use std::fs::read;
use std::fs::write;

use std::string::FromUtf8Error;

use std::process::Command;


fn refresh_tmux() {
    Command::new("tmux")
        .arg("refresh-client")
        .spawn()
     .expect("Failed to start process");
}


fn read_file(filename: &str) -> Result<String, FromUtf8Error> {
    let content = read(filename).unwrap();
    return String::from_utf8(content);
}

fn write_file(filename: &str, content: &str) -> () {
    write(filename, content).unwrap();
}

fn read_env_variables(keys: &[&str]) -> HashMap<String, String> {
    // snatched from
    // https://www.perplexity.ai/search/rust-get-env-variables-0QlZdWpaQuGXp.HG60dwCA#3

    let mut env_map = HashMap::new();

    for key in keys {
        if let Ok(value) = env::var(key) {
            env_map.insert(key.to_string(), value);
        } else {
            env_map.insert(key.to_string(), "".to_string());
            // println!("Environment variable {} is not set", key);
        }
    }

    env_map
}

fn update_tmux_display(os_cloud: &str, kubecfg: &str) {
    write_file("/tmp/._kubeconfig", kubecfg);
    write_file("/tmp/._openstack_cloud", os_cloud);

    refresh_tmux();
}

fn print_shortened_path(path: &str, home: &str, color: &str, no_color: &str, in_container: bool) {
    let mut prefix : String = "".to_string();
    if in_container {
        prefix += "NOT_HOST_ENV: ";
    }

    // let mut tokens: Vec<&str> = path.split('/').collect();
    let mut tokens: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
    if home.len() > 0 && path.starts_with(home) {
        prefix += "~/";

        if path.len() == home.len() {
            tokens.clear();
        }
        else {
            if tokens.len() > 3 {
                tokens.drain(0..3);
            }
        }
    }
    // println!("prefix: {}", prefix);
    // println!("tokens: {:?}", tokens);

    // if we do not cast this will end up an uint
    // this will result in a crash if a substractions
    // results in a value less than 0.
    let tokens_len = tokens.len() as i8;
    let mut idx = 0;
    for t in &mut tokens {
        // println!("idx: {}", idx);
        // println!("tokens_len: {}", tokens_len);

        if idx == (tokens_len -2) {
            continue;
        }
        if idx == tokens_len -1 {
            break;
        }
        // println!("after len.");

        if t.len() > 1 {
            if t.starts_with(".") {
                t.truncate(2);
            } else {
                t.truncate(1);
            }
        }
        idx += 1;
    }
    // println!("tokens: {:?}", tokens);
    let joined = tokens.join("/");
    // println!("joined: {}", joined);
    print!("{}{}{}{}", color, prefix, joined, no_color);
}

fn main() {

    let env_keys = ["HOME", "PWD", "NOT_HOST_ENV",
                    "OS_CLOUD", "KUBECONFIG", "GREEN",
                    "BLUE", "NC", "VIRTUAL_ENV"];
    let env = read_env_variables(&env_keys);
    let pwd = env["PWD"].as_str();
    let venv = env["VIRTUAL_ENV"].as_str();
    let home = env["HOME"].as_str();
    let blue = env["BLUE"].as_str();
    let green = env["GREEN"].as_str();
    let no_color = env["NC"].as_str();

    let not_host_env = env["NOT_HOST_ENV"].as_str();
    let in_container = if not_host_env.len() == 0 { false } else  { true };

    print_shortened_path(pwd, home, green, no_color, in_container);
    if venv.len() > 0 {
        print!(" (");
        print_shortened_path(venv, home, blue, no_color, in_container);
        print!(")");
    }
    println!("\n$ ");

    update_tmux_display(env["OS_CLOUD"].as_str(), env["KUBECONFIG"].as_str());

}

