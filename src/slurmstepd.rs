use std::error::Error;
use nix::unistd::{Uid, setfsuid};
//use std::sync::{Arc, Mutex};

use slurm_spank::{SpankHandle, spank_log_verbose};

use crate::{
    SpankSkyBox,
    job_get_info,
    is_skybox_enabled,
    //is_task_0,
    remove_folders,
    run_set_info,
    setup_folders,
    setup_privileged_folders,
    task_set_info,
};
use crate::args::*;
use crate::config::*;
use crate::environment::*;
use crate::podman::*;

#[allow(unused_variables)]
pub(crate) fn slurmstepd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("INIT");
    let _ = register_plugin_args(spank)?;
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("POST_OPT");
    let _ = load_plugin_args(plugin, spank)?;

    //let mut search_paths = vec![String::from("/etc/edf")];

    let user_uid = spank.job_uid()?;
    let old_uid = setfsuid(Uid::from(user_uid));
    spank_log_verbose!("JOB_UID:{user_uid}");
    spank_log_verbose!("OLD_JOB_UID:{old_uid}");
    let _ = load_environment(plugin, spank)?;
    let _ = setfsuid(Uid::from(old_uid));
    
    //let _ = set_remaining_default_args(plugin)?;

    if ! is_skybox_enabled(plugin, spank) {
        return Ok(());
    }

    let _ = job_get_info(plugin, spank)?;

    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_user_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("USER_INIT");
    /*
    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );
    */

    render_user_config(plugin, spank)?;
    update_edf_defaults_via_config(plugin)?;
    let _ = run_set_info(plugin, spank)?;
    setup_folders(plugin, spank)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init_privileged(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //spank_log_verbose!("TASK_INIT_PRIVILEGED");
    setup_privileged_folders(plugin, spank)
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("TASK_INIT");
    let _ = task_set_info(plugin, spank)?;

    podman_pull_once(plugin, spank)?;

    /*
    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );
    */
   
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("EXIT");

    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );

    remove_folders(plugin, spank)?;

    Ok(())
}
