use std::{path::PathBuf, str::FromStr};

use difftest::{initialize_backends, run_diff_test, Source};

#[test]
fn correct_mir() {
    let config = config::load("tests/config.toml");
    let backends = initialize_backends(config);

    let results = run_diff_test(
        &Source::File(PathBuf::from_str("tests/inputs/simple.rs").unwrap()),
        backends,
    );
    println!("{}", results);
    assert!(results.all_same());
    assert!(results["llvm"]
        .as_ref()
        .is_ok_and(|output| output.status.success() && output.stdout == "5\n"))
}

#[test]
fn invalid_mir() {
    let config = config::load("tests/config.toml");
    let backends = initialize_backends(config);

    let results = run_diff_test(
        &Source::File(PathBuf::from_str("tests/inputs/invalid_mir.rs").unwrap()),
        backends,
    );
    println!("{}", results);
    assert!(results.all_same());
    assert!(results["miri"].is_err());
    assert_eq!(results.has_ub(), Some(false));
}

#[test]
fn ub() {
    let config = config::load("tests/config.toml");
    let backends = initialize_backends(config);

    let results = run_diff_test(
        &Source::File(PathBuf::from_str("tests/inputs/ub.rs").unwrap()),
        backends,
    );
    println!("{}", results);
    assert_eq!(results.has_ub(), Some(true));
}
