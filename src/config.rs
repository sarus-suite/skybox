use std::error::Error;
use serde::{Deserialize, Serialize};
//use std::collections::HashMap;
//use std::process::Command;

use slurm_spank::{
    SpankHandle,
    spank_log_error,
};

use raster::{expand_vars_string};

use crate::{
    SpankSkyBox,
    get_job_env,
    get_plugin_name,
    plugin_err,
};

const CONFIG_FILE: &str = "/etc/sarus/skybox.conf";

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct RawConfig {
    enabled: Option<bool>,
    parallax_imagestore: Option<String>,
    parallax_mount_program: Option<String>,
    parallax_path: Option<String>,
    podman_module: Option<String>,
    podman_path: Option<String>,
    podman_tmp_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SkyBoxConfig {
    #[serde(default = "get_default_enabled")]
    pub(crate) enabled: bool,
    #[serde(default = "get_default_parallax_imagestore")]
    pub(crate) parallax_imagestore: String,
    #[serde(default = "get_default_parallax_mount_program")]
    pub(crate) parallax_mount_program: String,
    #[serde(default = "get_default_parallax_path")]
    pub(crate) parallax_path: String,
    #[serde(default = "get_default_podman_module")]
    pub(crate) podman_module: String,
    #[serde(default = "get_default_podman_path")]
    pub(crate) podman_path: String,
    #[serde(default = "get_default_podman_tmp_path")]
    pub(crate) podman_tmp_path: String,
}

fn get_default_enabled() -> bool {
    return false;
}

fn get_default_parallax_imagestore() -> String {
    return String::from("");
}

fn get_default_parallax_mount_program() -> String {
    return String::from("");
}

fn get_default_parallax_path() -> String {
    return String::from("parallax");
}

fn get_default_podman_module() -> String {
    return String::from("hpc");
}

fn get_default_podman_path() -> String {
    return String::from("podman");
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
            parallax_imagestore: match r.parallax_imagestore {
                Some(s) => s,
                None => get_default_parallax_imagestore(),
            },
            parallax_mount_program: match r.parallax_mount_program {
                Some(s) => s,
                None => get_default_parallax_mount_program(),
            },
            parallax_path: match r.parallax_path {
                Some(s) => s,
                None => get_default_parallax_path(),
            },
            podman_module: match r.podman_module {
                Some(s) => s,
                None => get_default_podman_module(),
            },
            podman_path: match r.podman_path {
                Some(s) => s,
                None => get_default_podman_path(),
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
        parallax_imagestore: None,
        parallax_mount_program: None,
        parallax_path: None,
        podman_module: None,
        podman_path: None,
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

/*
fn get_job_env(spank: &mut SpankHandle) -> HashMap<String,String> {
    
    let mut user_env = HashMap::new();

    let jobenv = match spank.job_env() {
        Ok(j) => j,
        Err(_) => {
            return user_env;
        }
    };
    
    for e in jobenv.iter() {
        let mut split = e.split("=");
        
        let size = split.clone().count();
        if size < 2 || size > 3 {
            continue;
        }
        
        let k = split.next().unwrap();
        let v = split.next().unwrap();
        user_env.insert(String::from(k),String::from(v));
    }

    user_env
}
*/

/*
fn expand_vars_string(input: String, env: &HashMap<String,String>) -> String {

    let output = Command::new("bash")
    .arg("-c")    
    .arg(format!("echo -n {}",&input))
    .env_clear()
    .envs(env)
    .output();

    let stdout = match output {
        Ok(o) => o.stdout,
        Err(_) => vec![],
    };

    let out = match str::from_utf8(&stdout) {
        Ok(o) => o,
        Err(_) => "",
    };

    String::from(out)
}
*/

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

    let r = load_raw_config(config_file_path.clone());
    let c = SkyBoxConfig::from(r);

    // Set Config
    match setup_config(&c, plugin) {
        Ok(_) => {},
        Err(e) => {
            spank_log_error!("{} at {}", e, config_file_path);
            spank_log_error!("[{}] plugin is disabled", get_plugin_name());
        },
    }

    c
}

pub(crate) fn setup_config(config: &SkyBoxConfig, plugin: &mut SpankSkyBox) -> Result<(), Box<dyn Error>> {
    plugin.config = config.clone();

    if config.parallax_imagestore == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find parallax_imagestore"); 
    }
    
    if config.parallax_mount_program == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find parallax_mount_program"); 
    }
    
    if config.parallax_path == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find parallax_path"); 
    }
    
    if config.podman_module == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find podman_module"); 
    }
    
    if config.podman_path == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find podman_path"); 
    }
    
    if config.podman_tmp_path == "" {
        plugin.config.enabled = false;
        return plugin_err("cannot find podman_tmp_path"); 
    }

    Ok(())
}

pub(crate) fn render_user_config(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
   
    let ue = &Some(get_job_env(spank));
    let config = plugin.config.clone();

    let user_config = SkyBoxConfig {
        enabled: config.enabled,
        parallax_imagestore: expand_vars_string(config.parallax_imagestore, ue)?,
        parallax_mount_program: expand_vars_string(config.parallax_mount_program, ue)?,
        parallax_path: expand_vars_string(config.parallax_path, ue)?,
        podman_module: expand_vars_string(config.podman_module, ue)?,
        podman_path: expand_vars_string(config.podman_path, ue)?,
        podman_tmp_path: expand_vars_string(config.podman_tmp_path, ue)?,
    };

    match setup_config(&user_config, plugin) {
        Ok(_) => {},
        Err(e) => {
            spank_log_error!("{} when expanding variables", e);
            spank_log_error!("[{}] plugin is disabled", get_plugin_name());
            return plugin_err("cannot render user configuration");
        },
    }

    Ok(())
}
