use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::sync::OnceLock;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use regex::Regex;

mod logging;


struct Args {
    path: String,
    patterns: Vec<OsString>,

}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut path = None;
    let mut patterns: Vec<OsString> = Vec::new();
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('e') | Long("patterns") => {
                patterns = parser.values()?.collect();
            }

            Value(val) if path.is_none() => {
                path = Some(val.string()?);
            }
            Long("help") => {
                debug!("Usage: filestile --patterns PATTERNS PATH");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        patterns: if patterns.is_empty() {
            return Err("missing option --patterns".into());
        } else {
            patterns
        },
        path: path.ok_or("missing argument PATH")?,
    })
}

#[derive(Debug)]
struct File {
    name: String,
    ts: SystemTime,
}

fn main() -> Result<(), lexopt::Error> {

    let mut last_matches: HashMap<String, File> = HashMap::new();

    let args = parse_args()?;
    let path_to_search = args.path;

    debug!("path_to_search: {}", path_to_search);
    debug!("patterns: {:?}", args.patterns);

    static RE: OnceLock<Regex> = OnceLock::new();
    let patterns = args.patterns
        .iter()
        .map(|os| "(".to_owned() + &os.to_string_lossy() + ")")
        .collect::<Vec<_>>()
        .join("|");
    let re = RE.get_or_init(|| Regex::new(&patterns).unwrap());
    debug!("re: {:?}", re);

    println!("");

    match fs::read_dir(&path_to_search) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            continue;
                        }

                    debug!("--------------------------------");

                    let p = entry.path().to_string_lossy().to_string();

                    if re.is_match(&p) {
                        debug!("Matched: {}", p);
                    } else {
                        // debug!("No match: {}\n---", p);
                        continue;
                    }

                    let created_nsec = metadata.created();

                    if let Ok(created_nsec) = created_nsec {
                        let _created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                        debug!("created @{}",  _created);

                        let caps = re.captures(&p).unwrap();
                        let shared_fname_section = caps.get(0).unwrap().as_str();

                        if let Some(file) = last_matches.get(shared_fname_section) {
                            debug!("File already saved.");
                            if file.ts.clone() > created_nsec {
                                log_info!("Current file older, continuing.");
                                log_info!("Removing previous entry.");
                                last_matches.retain(|key, _| re.is_match(key));
                            } else {
                                debug!("Current file newer, skipping.");
                                continue;
                            }
                        } else {
                            debug!("None @{}: {}", p, shared_fname_section);
                        }

                        last_matches.insert(shared_fname_section.to_string(), File { name: p, ts: created_nsec } );

                    }


                    }
                 }
            }
        }
        Err(err) => log_err!("Error reading directory: {}", err),

    }

    debug!("last_matches: {:?}", last_matches);

    let keep: HashSet<String> = last_matches
        .values()
        .map(|file| file.name.clone())
        .collect();

    match fs::read_dir(path_to_search) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            continue;
                        }

                    let p = entry.path().to_string_lossy().to_string();


                    if !keep.contains(&p) {
                        let _ = fs::remove_file(&p);
                        log_info!("Deleted {}.", p);
                        println!("---");
                    }

                    }
                }
            }
        }
        Err(err) => log_err!("Error reading directory: {}", err),
    }

    Ok(())
}
