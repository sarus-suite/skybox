use std::error::Error;

use slurm_spank::{
    SpankHandle,
    //spank_log_user,
};

use crate::SpankSkyBox;
use crate::args::*;
use crate::environment::*;

#[allow(unused_variables)]
pub(crate) fn srun_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let r = register_plugin_args(spank)?;
    //spank_log_user!("INIT");
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn srun_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //spank_log_user!("POST_OPT");

    let _ = load_plugin_args(plugin, spank)?;
    let _ = load_environment(plugin, spank)?;
    let _ = set_remaining_default_args(plugin)?;
    /*

    spank_log_user!("computed context:");
    spank_log_user!(
        "{}",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );
    */

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn srun_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //spank_log_user!("EXIT");

    Ok(())
}
