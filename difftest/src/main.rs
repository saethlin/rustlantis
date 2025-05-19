#![feature(iter_intersperse)]

use std::{
    io::{self, Read},
    process::ExitCode,
};

use clap::{Arg, Command};
use difftest::{backends, run_diff_test};
use log::{debug, error, info};

fn main() -> ExitCode {
    env_logger::init();

    let matches = Command::new("difftest")
        .arg(Arg::new("file").required(true).env("DIFFTEST_FILE"))
        .get_matches();
    let source = matches.get_one::<String>("file").expect("required");
    let reduce = match std::env::var("DIFFTEST_REDUCE")
        .as_ref()
        .map(String::as_str)
    {
        Ok("1" | "true" | "yes") => true,
        Err(_) | Ok(_) => false,
    };

    let config_path = std::env::var("RUSTLANTIS_CONFIG").unwrap_or("config.toml".to_string());
    let config = config::load(config_path);
    let backends = backends::from_config(config);

    let code = if source == "-" {
        let mut code = String::new();
        io::stdin()
            .read_to_string(&mut code)
            .expect("can read source code from stdin");
        code
    } else {
        std::fs::read_to_string(&source).expect("is valid path")
    };

    info!(
        "Difftesting {} with {}",
        source,
        backends
            .keys()
            .map(String::as_str)
            .intersperse(", ")
            .collect::<String>()
    );

    let results = run_diff_test(&code, backends);
    if reduce {
        // The miri run must be good.
        let miri_result = results.miri_result().unwrap();
        if miri_result.is_err() {
            info!("Miri did not pass, so this input must not be interesting");
            debug!("{:?}", miri_result);
            return ExitCode::FAILURE;
        }
        // And we need something different.
        if results.all_same() {
            info!("All backends behaved the same, so this input must not be interesting");
            ExitCode::FAILURE
        } else {
            info!("Miri passed but another backend behaved differently");
            ExitCode::SUCCESS
        }
    } else {
        if results.all_same() && results.all_success() {
            info!("{} is all the same", source);
            debug!("{}", results);
            ExitCode::SUCCESS
        } else {
            let results = results.to_string();
            error!("{} didn't pass:\n{results}", source);
            ExitCode::FAILURE
        }
    }
}
