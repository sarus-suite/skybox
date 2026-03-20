use nix::unistd::{Uid, setfsuid};
use std::error::Error;

use slurm_spank::SpankHandle;

use crate::args::*;
use crate::config::*;
use crate::container::*;
use crate::edf::*;
use raster::*;
//use crate::skybox_log_context;
use crate::sync::*;
use crate::{
    SpankSkyBox, VERSION, cleanup_fs_local, is_skybox_enabled, job_get_info, plugin_err,
    remote_unset_env_vars, run_set_info, setup_folders, setup_privileged_folders, skybox_log_error,
    skybox_log_info, task_set_info,
};

#[allow(unused_variables)]
pub(crate) fn slurmstepd_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    match slurmstepd_load_config(plugin, spank) {
        Ok(_) => (),
        Err(e) => {
            //do not print anything if configuration is fine, but plugin is disabled.
            return Ok(());
        }
    }

    skybox_log_info!("version v{}", VERSION);
    let _ = register_plugin_args(spank)?;
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //skybox_log_verbose!("INIT_POST_OPT");
    let _ = load_plugin_args(plugin, spank)?;
    if !plugin.config.skybox_enabled {
        return Ok(());
    }

    let user_uid = spank.job_uid()?;
    let old_uid = setfsuid(Uid::from(user_uid));
    let _ = load_edf(plugin, spank)?;
    //update_config_by_user(&mut plugin.config, plugin.edf.clone().unwrap())?;
    let _ = setfsuid(Uid::from(old_uid));

    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }

    let _ = job_get_info(plugin, spank)?;

    remote_unset_env_vars(plugin, spank)?;

    //skybox_log_context(plugin);

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
    //skybox_log_verbose!("USER_INIT");

    //skybox_log_context(plugin);

    match render_user_job_config(plugin, spank) {
        Ok(_) => (),
        Err(e) => {
            //do not print anything if configuration is fine, but plugin is disabled.
            return Ok(());
        }
    }

    //update_edf_defaults_via_config(plugin)?;
    //update_config_by_user(&mut plugin.config, plugin.edf.clone().unwrap())?;

    let _ = run_set_info(plugin, spank)?;
    setup_folders(plugin, spank)?;
    modify_edf_for_sbatch(plugin, spank)?;

    //skybox_log_context(plugin);

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
    //skybox_log_verbose!("TASK_INIT_PRIVILEGED");
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
    //skybox_log_verbose!("TASK_INIT");
    let _ = task_set_info(plugin, spank)?;

    sync_tracking(plugin, spank)?;
    sync_podman_pull(plugin, spank)?;
    sync_podman_start(plugin, spank)?;
    container_join(plugin, spank)?;
    container_wait_cwd(plugin, spank)?;
    container_import_env(plugin, spank)?;
    container_set_workdir(plugin, spank)?;
    //container_wait_entrypoint_handover(plugin, spank)?;

    //skybox_log_context(plugin);

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
    //skybox_log_verbose!("TASK_EXIT");
    let _ = task_set_info(plugin, spank)?;
    let _ = run_set_info(plugin, spank)?;

    //skybox_log_context(plugin);

    sync_podman_stop(plugin, spank)?;

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_skybox_enabled(plugin, spank) {
        return Ok(());
    }
    //skybox_log_verbose!("EXIT");

    //skybox_log_context(plugin);

    cleanup_fs_local(plugin, spank)?;
    sync_cleanup_fs_shared(plugin, spank)?;

    Ok(())
}

fn slurmstepd_load_config(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let config_path = resolve_config_path(spank);

    // do not fail on variable expansion -> &Some(false)
    let config = match load_config_path(config_path, VarExpand::Try, &None) {
        Ok(cfg) => cfg,
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("plugin is disabled");
            return plugin_err("plugin is disabled");
        }
    };

    // Set Config
    match setup_config(&config, plugin) {
        Ok(_) => {}
        Err(e) => {
            skybox_log_error!("{}", e);
            skybox_log_error!("plugin is disabled");
        }
    }

    if !plugin.config.skybox_enabled {
        return plugin_err("plugin is disabled");
    }

    Ok(())
}
