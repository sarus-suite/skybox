use std::error::Error;

use slurm_spank::SpankHandle;

use crate::SpankSkyBox;
use crate::args::*;
use crate::edf::*;

#[allow(unused_variables)]
pub(crate) fn srun_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let r = register_plugin_args(spank)?;
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn srun_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let _ = load_plugin_args(plugin, spank)?;
    let _ = load_edf(plugin, spank)?;
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
