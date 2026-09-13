use std::io::Read;
use std::path::PathBuf;

use afmt_core::Config;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "TODO")]
struct Cli {
    /// Path to the file to parse; reads from stdin if not provided
    file: Option<PathBuf>,

    /// Dump parse tree
    #[clap(long)]
    dump_tree: bool,
}

fn main() {
    let cli = Cli::parse();
    let code = match &cli.file {
        Some(path) => std::fs::read_to_string(path).unwrap(),
        None => {
            let mut code = String::new();
            std::io::stdin().read_to_string(&mut code).unwrap();
            code
        }
    };
    let cfg = Config {
        dump_tree: cli.dump_tree,
        ..Config::default()
    };
    let out = match afmt_core::format_source_code(&cfg, &code) {
        Ok(out) => out,
        Err(errors) => {
            for error in errors {
                println!("{:?}", error);
            }
            return;
        }
    };
    println!("{}", out);
}
