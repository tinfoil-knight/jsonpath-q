use jsonpath_q::interpret;

use std::io::Read;
use std::{fs, io};

use argh::FromArgs;

#[derive(FromArgs)]
/// Note: You can also stdin to provide input. Eg: cat data.json | jsonpath-q -q <query>
struct Config {
    /// query eg: "$['foo'].[1]"
    #[argh(option, short = 'q')]
    query: String,

    /// filepath
    #[argh(option, short = 'f')]
    filepath: Option<String>,
}

fn main() {
    let config: Config = argh::from_env();

    let mut input = String::new();

    if let Some(path) = config.filepath {
        input = fs::read_to_string(path).expect("failed to read the file")
    } else {
        io::stdin()
            .read_to_string(&mut input)
            .expect("failed to read stdin");
    }

    if input.trim().is_empty() {
        eprintln!("provided file data or stdin is empty");
        std::process::exit(1);
    }

    let result = interpret(&input, &config.query).expect("failed to process query");
    println!("{}", serde_json::to_string_pretty(&result).unwrap())
}
