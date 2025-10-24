use std::error::Error;

use slurm_spank::{SpankHandle, spank_log_verbose};

use crate::SpankSkyBox;
use crate::args::*;

#[allow(unused_variables)]
pub(crate) fn slurmstepd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let r = register_plugin_args(spank)?;
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let _ = load_plugin_args(plugin, spank)?;
    let _ = set_remaining_default_args(plugin)?;

    spank_log_verbose!("{}: computed args:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init_privileged(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
