use ini::Ini;
use std::env;

mod logging;

// USAGE: read-ini-setting <CONF_FILE> <ITEM> [<SECTION>]
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let filename = args.get(0).unwrap().to_owned();
    let conf = Ini::load_from_file(filename).unwrap();

    let mut section_name = "default".to_string();
    if args.len() > 2 {
        section_name = args.get(2).unwrap().to_owned();
        debug!("overrode section_name to: {}", section_name);
    }
    let section = conf.section(Some(section_name)).unwrap();
    debug!("section: {:?}", section);

    let key = args.get(1).unwrap().to_owned();
    let value = section.get(key).unwrap();

    print!("{}", value);

    // // Iterate over all sections and properties
    // for (sec, prop) in &conf {
    //     println!("Section: {:?}", sec);
    //     for (key, value) in prop.iter() {
    //         println!("{} = {}", key, value);
    //     }
    // }
}
