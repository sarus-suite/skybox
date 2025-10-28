use serde::{Deserialize, Serialize};

use slurm_spank::{SpankHandle};

use crate::SpankSkyBox;

const CONFIG_FILE: &str = "/etc/sarus/skybox.conf";

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RawConfig {
    enabled: Option<bool>,
    podman_tmp_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SkyBoxConfig {
    #[serde(default = "get_default_enabled")]
    pub(crate) enabled: bool,
    #[serde(default = "get_default_podman_tmp_path")]
    pub(crate) podman_tmp_path: String,
}

fn get_default_enabled() -> bool {
    return false;
}

fn get_default_podman_tmp_path() -> String {
    return String::from("/dev/shm");
}

impl From<RawConfig> for SkyBoxConfig {
    fn from(r: RawConfig) -> Self {
        SkyBoxConfig {
            enabled: match r.enabled {
                Some(s) => s,
                None => get_default_enabled(),
            },
            podman_tmp_path: match r.podman_tmp_path {
                Some(s) => s,
                None => get_default_podman_tmp_path(),
            },
        }
    }
}

fn load_raw_config(filepath: String) -> RawConfig {
    let path_str = filepath.as_str();

    let empty = RawConfig {
        enabled: None,
        podman_tmp_path: None,
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

pub(crate) fn load_config(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> SkyBoxConfig {

    
    let plugin_argv = spank.plugin_argv();

    let mut config_file: Option<String> = None;
    
    for args in plugin_argv.iter() {
        for arg in args.iter() {
            let mut fields = arg.split("=");
            let key = match fields.next() {
                Some(k) => k,
                None => continue,
            };
            let value = match fields.next() {
                Some(v) => v,
                None => continue,
            };

            if key == "config_path" {
                config_file = Some(String::from(value));
            }
        }
    }
    
    let config_file_path = match config_file {
        None => String::from(CONFIG_FILE),
        Some(cfg) => cfg,
    };

    let r = load_raw_config(config_file_path);
    let c = SkyBoxConfig::from(r);

    // Set Config
    setup_config(&c, plugin);

    c
}

pub(crate) fn setup_config(config: &SkyBoxConfig, plugin: &mut SpankSkyBox) -> () {
    //plugin.enabled = config.enabled;
    plugin.config = config.clone();
}
