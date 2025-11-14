use std::error::Error;

use slurm_spank::{SpankHandle, spank_log_info};

use crate::{SpankSkyBox, plugin_string};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(unused_variables)]
pub(crate) fn slurmd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let msg = plugin_string(format!("version v{}", VERSION).as_str());
    spank_log_info!("{msg}");
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
