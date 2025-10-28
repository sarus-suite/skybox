use std::error::Error;
use nix::unistd::{Uid, setfsuid};

use slurm_spank::{SpankHandle, spank_log_verbose};

use crate::{
    SpankSkyBox,
    job_get_info,
    is_skybox_enabled,
    privileged_setup_folders,
    setup_folders,
};
use crate::args::*;
use crate::environment::*;

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

    let job = plugin.job.clone().unwrap();
    let config = plugin.config.clone();
    let container_name = format!("skybox_{}.{}", job.jobid, job.stepid);
    let podman_tmp_path = format!("{}/{}", config.podman_tmp_path, container_name);
    setup_folders(podman_tmp_path, plugin, spank)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("TASK_INIT");
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("EXIT");
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init_privileged(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //spank_log_verbose!("TASK_INIT_PRIVILEGED");
    Ok(privileged_setup_folders(plugin, spank)?)
}
