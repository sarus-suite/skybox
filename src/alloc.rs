use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use slurm_spank::{SpankHandle, spank_log_user};

use crate::args::*;
use crate::config::*;
use crate::edf::*;
use crate::{SpankSkyBox, plugin_err, skybox_log_error};
use raster::*;

#[allow(unused_variables)]
pub(crate) fn alloc_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    match alloc_load_config(plugin, spank) {
        Ok(_) => (),
        Err(e) => {
            //do not print anything if configuration is fine, but plugin is disabled.
            return Ok(());
        }
    }

    register_plugin_args(spank)?;
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    load_plugin_args(plugin, spank)?;
    load_edf(plugin, spank)?;
    update_config_by_user(&mut plugin.config, plugin.edf.clone().unwrap())?;
    set_remaining_default_args(plugin)?;

    //skybox_log_context(plugin);
    sbatch_warn_msg(plugin, spank);
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub(crate) fn sbatch_warn_msg(plugin: &mut SpankSkyBox, _spank: &mut SpankHandle) -> () {
    if plugin.args.edf.is_none() {
        return ();
    };

    let prog = env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
        .unwrap();

    let unpadded = format!("\"{}\" is still considered experimental", prog);
    let warnmsg = format!(
        "
--------------------------------------------------------------------------------
| Use of the \"--edf\" option for {:<width$}|
| and could result in unexpected behavior.                                     |
| Use of \"--edf\" is currently only recommended for the \"srun\" command.         |
|                                                                              |
| Please read carefully the Container Engine page on the CSCS Knowledge Base.  |
--------------------------------------------------------------------------------
",
        unpadded,
        width = 47
    );

    spank_log_user!("{warnmsg}");
}

fn alloc_load_config(
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
