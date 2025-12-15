use std::error::Error;

use slurm_spank::SpankHandle;

use crate::config::resolve_config_path;
use crate::{SpankSkyBox, VERSION, plugin_err, skybox_log_debug, skybox_log_info};
use raster::*;

fn slurmd_load_config(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let config_path = resolve_config_path(spank);

    // do not fail on variable expansion -> &Some(false)
    plugin.config = load_config_path(config_path, VarExpand::Try, &None)?;

    if !plugin.config.skybox_enabled {
        return plugin_err("plugin is disabled");
    }

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    match slurmd_load_config(plugin, spank) {
        Ok(_) => (),
        Err(e) => {
            skybox_log_debug!("{e}");
            return Ok(());
        }
    }

    skybox_log_info!("version v{}", VERSION);
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
