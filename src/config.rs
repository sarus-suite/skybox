use std::error::Error;
use std::path::PathBuf;

use slurm_spank::SpankHandle;

use raster::*;

use crate::{SpankSkyBox, get_job_env, plugin_err, skybox_log_error};

pub(crate) fn resolve_config_path(spank: &mut SpankHandle) -> Option<PathBuf> {
    let plugin_argv = spank.plugin_argv();

    let mut config_path: Option<PathBuf> = None;

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
                config_path = Some(PathBuf::from(value));
            }
        }
    }

    config_path
}

pub(crate) fn plugin_enabled_in_config(
    _plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<bool, Box<dyn Error>> {
    let config_path = resolve_config_path(spank);

    // Do not expand variables
    let config = load_config_path(config_path, VarExpand::Never, &None)?;

    return Ok(config.skybox_enabled);
}

/*
pub(crate) fn load_config(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<Config, Box<dyn Error>> {
    let config_path = resolve_config_path(spank);

    let config = raster::load_config_path(config_path, &Some(false), &None)?;

    // Set Config
    match setup_config(&config, plugin) {
        Ok(_) => {}
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("plugin is disabled");
        }
    }

    Ok(config)
}
*/

pub(crate) fn setup_config(
    config: &Config,
    plugin: &mut SpankSkyBox,
) -> Result<(), Box<dyn Error>> {
    plugin.config = config.clone();

    if config.parallax_imagestore == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find parallax_imagestore");
    }

    if config.parallax_mount_program == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find parallax_mount_program");
    }

    if config.parallax_path == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find parallax_path");
    }

    if config.podman_module == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find podman_module");
    }

    if config.podman_path == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find podman_path");
    }

    if config.podman_tmp_path == "" {
        plugin.config.skybox_enabled = false;
        return plugin_err("cannot find podman_tmp_path");
    }

    Ok(())
}

pub(crate) fn render_user_job_config(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let config_path = resolve_config_path(spank);
    let je = &Some(get_job_env(spank));
    //let job_config = raster::load_config_path(config_path, &Some(true), &je)?;

    // force variable expansion -> &Some(true)
    let job_config = match load_config_path(config_path, VarExpand::Must, &je) {
        Ok(cfg) => cfg,
        Err(e) => {
            plugin.config.skybox_enabled = false;
            skybox_log_error!("{}", e);
            skybox_log_error!("Error on configuration loading");
            skybox_log_error!("plugin is disabled");
            return plugin_err("plugin is disabled");
        }
    };

    match setup_config(&job_config, plugin) {
        Ok(_) => {}
        Err(e) => {
            skybox_log_error!("{} when expanding variables", e);
            skybox_log_error!("plugin is disabled");
            return plugin_err("cannot render user job configuration");
        }
    }

    Ok(())
}
