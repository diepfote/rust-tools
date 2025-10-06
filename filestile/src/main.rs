use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::sync::OnceLock;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use regex::Regex;

mod logging;

struct Args {
    dry_run: bool,
    path: String,
    pattern: String,
    match_group_indexes: Vec<i32>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut dry_run = false;
    let mut path = None;
    let mut pattern: String = "".to_string();
    let mut match_group_indexes: Vec<i32> = Vec::new();

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Long("dry-run") => {
                dry_run = true;
            }

            Short('e') | Long("pattern") => {
                if let Ok(value) = parser.value() {
                    let value_str = value.to_string_lossy();
                    pattern = value_str.to_string();
                }
            }

            Short('m') | Long("match-group-indexes") => {
                match_group_indexes = parser
                    .values()?
                    .filter_map(|osstr| osstr.to_string_lossy().parse::<i32>().ok())
                    .collect();
            }

            Value(val) if path.is_none() => {
                path = Some(val.string()?);
            }

            Short('h') | Long("help") => {
                println!(
                    "Usage: filestile --match-group-indexes INDEXES --pattern PATTERN -- PATH"
                );
                println!("Usage: filestile -m INDEXES -e PATTERN -- PATH");
                println!("e.g.: filestile -m 2 3 -e '.*(Blocksberg|Tina).*(Folge [0-9]+).*'  -- .");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        dry_run: dry_run,
        pattern: if pattern.is_empty() {
            return Err("missing option -e/--pattern".into());
        } else {
            pattern
        },
        match_group_indexes: if match_group_indexes.is_empty() {
            return Err("missing option -m/--match-group-indexes".into());
        } else {
            match_group_indexes
        },
        path: path.ok_or("missing argument PATH")?,
    })
}

// Extract match groups from captures and join them using " "
//
// e.g. if a file is:
//  /.../tmp.uaF4y1nMl0/Bibi & Tina -  Das sprechende Pferd (Folge 29) _ Hörspiel des Monats - DAS ZWEITPLATZIERTE....m4a
//  , then the pattern we should use is: '.*(Blocksberg|Tina).*(Folge [0-9]+).*'
//  and shared_fname ends up being: Tina Folge 29
fn get_shared_fname(path: &String, match_group_indexes: Vec<i32>, regexp: Regex) -> String {
    let re_captures = regexp.captures(path).unwrap();
    debug!("re_captures: {:?}", re_captures);

    let groups = match_group_indexes
        .iter()
        .filter_map(|&idx| {
            re_captures
                .get(idx.try_into().unwrap())
                .map(|match_group| match_group.as_str())
        })
        .collect::<Vec<&str>>();
    return groups.join(" ");
}

fn get_regex_pattern(pattern: String) -> Regex {
    debug!("pattern: {}", pattern);

    static RE: OnceLock<Regex> = OnceLock::new();
    return RE.get_or_init(|| Regex::new(&pattern).unwrap()).clone();
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
    let match_group_indexes = args.match_group_indexes;
    let pattern = args.pattern;

    debug!("path_to_search: {}", path_to_search);

    let regexp = get_regex_pattern(pattern);

    println!("");

    if let Ok(dir) = fs::read_dir(&path_to_search) {
        for entry in dir {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
                && let Ok(created_nsec) = metadata.created()
            {
                if metadata.is_dir() {
                    continue;
                }

                debug!("--------------------------------");

                let path = entry.path().to_string_lossy().to_string();

                if regexp.is_match(&path) {
                    debug!("Matched: {}", path);
                } else {
                    // debug!("No match: {}\n---", path);
                    continue;
                }

                let _created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                debug!("created @{}", _created);

                let shared_fname =
                    get_shared_fname(&path, match_group_indexes.clone(), regexp.clone());
                log_info!("shared_fname: {}", shared_fname);

                // Remember: we want to keep the oldest file
                if let Some(file) = last_matches.get(&shared_fname) {
                    debug!("File already saved.");
                    if file.ts.clone() > created_nsec {
                        log_info!("Current file older, continuing.");
                        log_info!("Removing previous entry: {}", file.name);
                        log_info!("Adding: {}", path);
                        println!("---");
                        last_matches.retain(|key, _| regexp.is_match(key));
                    } else {
                        debug!("Current file newer, skipping. Keeping: {}", file.name);
                        continue;
                    }
                } else {
                    debug!("Not saved yet: {}: shared_fname: {}", path, shared_fname);
                }

                last_matches.insert(
                    shared_fname.to_string(),
                    File {
                        name: path,
                        ts: created_nsec,
                    },
                );
            }
        }
    }
    debug!("last_matches: {:?}", last_matches);

    let keep: HashSet<String> = last_matches
        .values()
        .map(|file| file.name.clone())
        .collect();

    println!("---");

    if let Ok(dir) = fs::read_dir(&path_to_search) {
        for entry in dir {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
                && !metadata.is_dir()
            {
                {
                    let path = entry.path().to_string_lossy().to_string();

                    if !keep.contains(&path) {
                        if args.dry_run {
                            log_info!("Would delete: {}.", path);
                        } else {
                            let _ = fs::remove_file(&path);
                            log_info!("Deleted: {}.", path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
