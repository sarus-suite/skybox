use std::error::Error;

use slurm_spank::SpankHandle;

use crate::args::*;
use crate::config::*;
use crate::edf::*;
use crate::{SpankSkyBox, plugin_err, skybox_log_error};
use raster::*;

fn srun_load_config(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //Just load plain configuration to understand if plugin is enabled.
    match plugin_enabled_in_config(plugin, spank) {
        Ok(bool) => {
            if !bool {
                return plugin_err("plugin is disabled");
            }
        }
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("Error on configuration loading");
            skybox_log_error!("plugin is disabled");
            return plugin_err("plugin is disabled");
        }
    }

    //Load and expand configuration.
    let config_path = resolve_config_path(spank);

    // force variable expansion -> &Some(true)
    let config = match load_config_path(config_path, VarExpand::Must, &None) {
        Ok(cfg) => cfg,
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("Error on configuration loading");
            skybox_log_error!("plugin is disabled");
            return plugin_err("plugin is disabled");
        }
    };

    // Set Config
    match setup_config(&config, plugin) {
        Ok(_) => {}
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("plugin is disabled");
        }
    }

    if !plugin.config.skybox_enabled {
        return plugin_err("plugin is disabled");
    }

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn srun_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    match srun_load_config(plugin, spank) {
        Ok(_) => (),
        Err(e) => {
            //do not print anything if configuration is fine, but plugin is disabled.
            return Ok(());
        }
    }

    let r = register_plugin_args(spank)?;
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn srun_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let _ = load_plugin_args(plugin, spank)?;
    if !plugin.config.skybox_enabled {
        return Ok(());
    }

    let _ = load_edf(plugin, spank)?;
    update_config_by_user(&mut plugin.config, plugin.edf.clone().unwrap())?;
    let _ = set_remaining_default_args(plugin)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn srun_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
