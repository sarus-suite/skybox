use std::error::Error;

use slurm_spank::SpankHandle;

use crate::args::*;
use crate::{SpankSkyBox, skybox_log_context};

#[allow(unused_variables)]
pub(crate) fn alloc_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    register_plugin_args(spank)?;
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    skybox_log_context(plugin);
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
