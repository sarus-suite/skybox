use nix::unistd::{Uid, setfsuid};
use std::error::Error;
//use std::sync::{Arc, Mutex};

use slurm_spank::{
    SpankHandle,
    //spank_log_user,
    spank_log_verbose,
};

use crate::args::*;
use crate::config::*;
use crate::container::*;
use crate::environment::*;
use crate::podman::*;
use crate::{
    SpankSkyBox,
    cleanup_fs_local,
    cleanup_fs_shared_once,
    is_skybox_enabled,
    job_get_info,
    //is_task_0,
    remote_unset_env_vars,
    run_set_info,
    setup_folders,
    setup_privileged_folders,
    task_set_info,
};

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
pub(crate) fn slurmstepd_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    spank_log_verbose!("INIT_POST_OPT");
    let _ = load_plugin_args(plugin, spank)?;

    //let mut search_paths = vec![String::from("/etc/edf")];

    let user_uid = spank.job_uid()?;
    let old_uid = setfsuid(Uid::from(user_uid));
    spank_log_verbose!("JOB_UID:{user_uid}");
    spank_log_verbose!("OLD_JOB_UID:{old_uid}");
    let _ = load_environment(plugin, spank)?;
    let _ = setfsuid(Uid::from(old_uid));

    //let _ = set_remaining_default_args(plugin)?;

    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }

    let _ = job_get_info(plugin, spank)?;

    remote_unset_env_vars(plugin, spank)?;

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
pub(crate) fn slurmstepd_user_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
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

    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );
    //podman_pull_once(plugin, spank)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init_privileged(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
    spank_log_verbose!("TASK_INIT_PRIVILEGED");
    setup_privileged_folders(plugin, spank)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
    spank_log_verbose!("TASK_INIT");
    let _ = task_set_info(plugin, spank)?;

    podman_pull_once(plugin, spank)?;
    //let pause = std::time::Duration::new(60,0);
    //std::thread::sleep(pause);
    podman_start_once(plugin, spank)?;
    //std::thread::sleep(pause);
    container_join(plugin, spank)?;
    container_wait_cwd(plugin, spank)?;
    container_import_env(plugin, spank)?;
    container_set_workdir(plugin, spank)?;
    //std::thread::sleep(pause);

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
pub(crate) fn slurmstepd_task_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
    spank_log_verbose!("TASK_EXIT");
    let _ = task_set_info(plugin, spank)?;
    podman_stop_once(plugin, spank)?;
    //let pause = std::time::Duration::new(60,0);
    //std::thread::sleep(pause);
    Ok(())
}

#[allow(unused_variables)]
#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
    spank_log_verbose!("EXIT");
    /*
    spank_log_verbose!("{}: computed context:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );
    */
    //podman_stop(plugin, spank)?;
    cleanup_fs_shared_once(plugin, spank)?;
    cleanup_fs_local(plugin, spank)?;

    Ok(())
}
