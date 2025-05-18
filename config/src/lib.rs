use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub fn load(path: impl AsRef<Path>) -> Config {
    let config = std::fs::read_to_string(path).unwrap();
    toml::from_str(&config).unwrap()
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub generation: GenerationConfig,
    pub backends: HashMap<String, BackendConfig>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum BackendConfig {
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

#[derive(Deserialize)]
pub struct GenerationConfig {
    /// Max. number of statements & declarations in a bb
    #[serde(default = "bb_max_len")]
    pub bb_max_len: usize,

    /// Max. number of switch targets in a SwitchInt terminator
    #[serde(default = "max_switch_targets")]
    pub max_switch_targets: usize,

    /// Max. number of BB in a function if RET is init (a Return must be generated)
    #[serde(default = "max_bb_count")]
    pub max_bb_count: usize,

    /// Max. number of BB in a function before giving up this function
    #[serde(default = "max_bb_count_hard")]
    pub max_bb_count_hard: usize,

    /// Max. number of functions in the program Call generator stops being a possible candidate
    #[serde(default = "max_fn_count")]
    pub max_fn_count: usize,

    /// Max. number of arguments a function can have
    #[serde(default = "max_args_count")]
    pub max_args_count: usize,

    /// Expected proportion of variables to be dumped
    #[serde(default = "var_dump_chance")]
    pub var_dump_chance: f32,
}

fn bb_max_len() -> usize {
    32
}

fn max_switch_targets() -> usize {
    8
}

fn max_bb_count() -> usize {
    50
}

fn max_bb_count_hard() -> usize {
    100
}

fn max_fn_count() -> usize {
    20
}

fn max_args_count() -> usize {
    12
}

fn var_dump_chance() -> f32 {
    0.5
}
