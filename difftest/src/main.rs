#![feature(iter_intersperse)]

use std::{
    collections::HashMap,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
    str::FromStr,
};

use clap::{Arg, Command};
use difftest::{
    backends::{Backend, Cranelift, Miri, GCC, LLUBI, LLVM},
    run_diff_test, Source,
};
use log::{debug, error, info};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    backends: HashMap<String, BackendConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
enum BackendConfig {
    Miri {
        toolchain: String,
        flags: Vec<String>,
    },
    LLVM {
        toolchain: String,
        flags: Vec<String>,
    },
    Cranelift {
        toolchain: String,
        flags: Vec<String>,
    },
    GCC {
        repo: String,
        flags: Vec<String>,
    },
    LLUBI {
        toolchain: String,
        llubi_path: String,
        flags: Vec<String>,
    },
}

fn main() -> ExitCode {
    env_logger::init();

    let matches = Command::new("difftest")
        .arg(Arg::new("file").required(true))
        .get_matches();
    let source = matches.get_one::<String>("file").expect("required");

    let config = std::fs::read_to_string("config.toml").unwrap();
    let v: toml::Value = toml::from_str(&config).unwrap();
    eprintln!("{:#?}", v["backends"]);
    let settings: Config = toml::from_str(&config).unwrap();

    let mut backends = HashMap::new();
    for (name, config) in settings.backends {
        let backend: Box<dyn Backend> = match config {
            BackendConfig::Miri { toolchain, flags } => {
                Box::new(Miri::from_rustup(toolchain, flags).unwrap())
            }
            BackendConfig::LLVM { toolchain, flags } => Box::new(LLVM::new(toolchain, flags)),
            BackendConfig::Cranelift { toolchain, flags } => {
                Box::new(Cranelift::from_rustup(toolchain, flags))
            }
            BackendConfig::GCC { repo, flags } => {
                Box::new(GCC::from_built_repo(repo, flags).unwrap())
            }
            BackendConfig::LLUBI {
                toolchain,
                llubi_path,
                flags,
            } => Box::new(LLUBI::new(toolchain, llubi_path, flags)),
        };
        backends.insert(name, backend);
    }

    let source = if source == "-" {
        let mut code = String::new();
        io::stdin()
            .read_to_string(&mut code)
            .expect("can read source code from stdin");
        Source::Stdin(code)
    } else {
        Source::File(PathBuf::from_str(source).expect("is valid path"))
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

    let results = run_diff_test(&source, backends);
    if results.all_same() && results.all_success() {
        info!("{} is all the same", source);
        debug!("{}", results);
        ExitCode::SUCCESS
    } else {
        let results = results.to_string();
        error!("{} didn't pass:\n{results}", source,);
        ExitCode::FAILURE
    }
}
