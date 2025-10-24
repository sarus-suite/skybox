use serde::{Deserialize, Serialize};

use slurm_spank::{SpankHandle, spank_log_user};

const CONFIG_FILE: &str = "/etc/sarus/skybox.conf";

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RawConfig {
    enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    #[serde(default = "get_default_enabled")]
    pub(crate) enabled: bool,
}

fn get_default_enabled() -> bool {
    return false;
}

impl From<RawConfig> for Config {
    fn from(r: RawConfig) -> Self {
        Config {
            enabled: match r.enabled {
                Some(s) => s,
                None => get_default_enabled(),
            },
        }
    }
}

fn load_raw_config(filepath: String) -> RawConfig {
    let path_str = filepath.as_str();

    let empty = RawConfig {
        enabled: None,
    };

    let toml_content = match std::fs::read_to_string(path_str) {
        Ok(c) => c,
        Err(_) => {
            return empty;
        }
    };

    let toml_value = match toml::from_str(&toml_content) {
        Ok(v) => v,
        Err(_) => {
            return empty;
        }
    };

    let r: RawConfig = toml_value;
    r
}

pub(crate) fn load_config(spank: SpankHandle) -> Config {

    
    //let plugin_argv = spank.plugin_argv();

    let config_file: Option<String> = None;
    /*
    for arg in plugin_argv.iter() {
        spank_log_user!("{}", arg);
    }
    */
    
    let config_file_path = match config_file {
        None => String::from(CONFIG_FILE),
        Some(cfg) => cfg,
    };

    let r = load_raw_config(config_file_path);
    let c = Config::from(r);
    c
}
