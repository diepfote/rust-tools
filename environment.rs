use std::collections::HashMap;
use std::env;

pub fn read_env_variables(keys: &[&str]) -> HashMap<String, String> {
    // snatched from
    // https://www.perplexity.ai/search/rust-get-env-variables-0QlZdWpaQuGXp.HG60dwCA#3

    let mut env_map = HashMap::new();

    for key in keys {
        if let Ok(value) = env::var(key) {
            env_map.insert(key.to_string(), value);
        } else {
            env_map.insert(key.to_string(), "".to_string());
        }
    }
    env_map
}
