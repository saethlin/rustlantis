#![feature(iter_intersperse)]

use std::{
    collections::HashMap,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
    str::FromStr,
};

use clap::{Arg, Command};
use config::BackendConfig;
use difftest::{
    backends::{Backend, Cranelift, Miri, GCC, LLUBI, LLVM},
    run_diff_test, Source,
};
use log::{debug, error, info};

fn main() -> ExitCode {
    env_logger::init();

    let matches = Command::new("difftest")
        .arg(Arg::new("file").required(true))
        .arg(
            Arg::new("reduce")
                .long("reduce")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();
    let source = matches.get_one::<String>("file").expect("required");
    let reduce = matches.get_flag("reduce");

    let config = config::load();

    let mut backends = HashMap::new();
    for (name, config) in config.backends {
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

    // FIXME: Read the source from disk here, so that no matter how many backends we run we always
    // read the code only once.
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
    if reduce {
        // The miri run must be good.
        let miri_result = results.miri_result().unwrap();
        if miri_result.is_err() {
            info!("Miri did not pass, so this input must not be interesting");
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
            error!("{} didn't pass:\n{results}", source,);
            ExitCode::FAILURE
        }
    }
}
