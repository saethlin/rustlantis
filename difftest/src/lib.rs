#![feature(iter_intersperse)]
#![feature(let_chains)]

pub mod backends;

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    path::PathBuf,
    time::Instant,
};

use backends::{Backend, CompExecError, ExecResult};
use colored::Colorize;
use log::{debug, log_enabled};

pub enum Source {
    File(PathBuf),
    Stdin(String),
}

impl Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::File(path) => f.write_str(&path.to_string_lossy()),
            Source::Stdin(_) => f.write_str("[stdin]"),
        }
    }
}

pub struct ExecResults {
    // Equivalence classes of exec results and backends
    results: HashMap<ExecResult, HashSet<String>>,
}

impl ExecResults {
    pub fn from_exec_results<'a>(map: impl Iterator<Item = (String, ExecResult)>) -> Self {
        //TODO: optimisation here to check if all results are equal directly, since most should be

        // Split execution results into equivalent classes
        let mut eq_classes: HashMap<ExecResult, HashSet<String>> = HashMap::new();

        'outer: for (name, result) in map {
            for (class_result, names) in &mut eq_classes {
                // Put into an existing equivalence class
                let eq = if let Ok(class_out) = class_result
                    && let Ok(out) = &result
                {
                    class_out.stdout == out.stdout
                } else {
                    result == *class_result
                };
                if eq {
                    names.insert(name.clone());
                    continue 'outer;
                }
            }

            // No equal execution result, make a new class
            eq_classes.insert(result.clone(), HashSet::from([name]));
        }

        Self {
            results: eq_classes,
        }
    }

    pub fn all_same(&self) -> bool {
        self.results.len() == 1
    }

    pub fn all_success(&self) -> bool {
        self.results.keys().all(|r| r.is_ok())
    }

    pub fn has_ub(&self) -> Option<bool> {
        self.miri_result().map(|result| {
            result.clone().is_err_and(|err| {
                err.0
                    .stderr
                    .to_string_lossy()
                    .contains("Undefined Behavior")
            })
        })
    }

    pub fn miri_result(&self) -> Option<&ExecResult> {
        self.results.iter().find_map(|(result, backends)| {
            if backends.contains("miri") {
                Some(result)
            } else {
                None
            }
        })
    }
}

impl std::ops::Index<&str> for ExecResults {
    type Output = ExecResult;

    fn index(&self, index: &str) -> &Self::Output {
        for (result, names) in &self.results {
            if names.contains(index) {
                return result;
            }
        }
        panic!("no result for {index}")
    }
}

impl fmt::Display for ExecResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (result, names) in &self.results {
            f.write_fmt(format_args!(
                "{} produced the following output:\n",
                names
                    .iter()
                    .map(String::as_str)
                    .intersperse(", ")
                    .collect::<String>()
                    .blue()
            ))?;
            match result {
                Ok(out) => {
                    f.write_fmt(format_args!("stdout:\n{}", out.stdout.to_string_lossy()))?;
                }
                Err(CompExecError(out)) => {
                    f.write_fmt(format_args!("status: {}\n", out.status))?;
                    f.write_fmt(format_args!(
                        "stdout:\n{}================\n",
                        out.stdout.to_string_lossy()
                    ))?;
                    f.write_fmt(format_args!(
                        "{}:\n{}================\n",
                        "stderr".red(),
                        out.stderr.to_string_lossy()
                    ))?;
                }
            }
        }
        Ok(())
    }
}

pub fn run_diff_test<'a>(
    source: &Source,
    backends: HashMap<String, Box<dyn Backend + 'a>>,
) -> ExecResults {
    let mut target_dir = None;
    let mut results = Vec::new();
    for (name, backend) in backends {
        let result = if log_enabled!(log::Level::Debug) {
            let time = Instant::now();
            let result = backend.execute(source, &mut target_dir);
            let dur = time.elapsed();
            debug!("{name} took {}s", dur.as_secs_f32());
            result
        } else {
            backend.execute(source, &mut target_dir)
        };
        results.push((name, result));
    }

    ExecResults::from_exec_results(results.into_iter())
}
