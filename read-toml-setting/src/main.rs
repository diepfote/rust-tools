use std::fs;
// use std::env;

mod logging;

use toml;

// USAGE: read-toml-setting <CONF_FILE> <ITEM> [<SECTION>]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string("/Users/florian.sorko/.config/personal/fh.conf")?;
    debug!("content: {:?}", content);

    let value: toml::Value = toml::from_str(&content)?;
    debug!("value: {:?}", value);

    if let Some(default) = value.get("default") {
        debug!("default: {:?}", default);
        if let Some(username) = default.get("username") {
            debug!("username: {}", username);
        }
    }

    // let args: Vec<String> = env::args().skip(1).collect();
    // for key in &args {
    //     match value.get(key) {
    //         Some(v) => println!("{}: {}", key, v),
    //         None => println!("Key '{}' not found", key),
    //     }
    // }

    Ok(())
}
