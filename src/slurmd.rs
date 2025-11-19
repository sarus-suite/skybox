use std::error::Error;

use slurm_spank::SpankHandle;

use crate::{SpankSkyBox, VERSION, skybox_log_info};

#[allow(unused_variables)]
pub(crate) fn slurmd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //let msg = plugin_string(format!("version v{}", VERSION).as_str());
    //spank_log_info!("{msg}");
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
