use std::error::Error;

use slurm_spank::SpankHandle;

use crate::SpankSkyBox;

#[allow(unused_variables)]
pub(crate) fn slurmd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
