use nix::libc::{gid_t, uid_t};
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::Permissions;
use std::os::unix::fs::{PermissionsExt, chown};
//use std::os::raw::c_int;
use std::path::Path;
//use std::sync::{Arc, Mutex};

use slurm_spank::{Plugin, SLURM_VERSION_NUMBER, SPANK_PLUGIN, SpankHandle, spank_log_error};

//use raster::mount::SarusMounts;
use crate::args::SkyBoxArgs;
use crate::config::SkyBoxConfig;
use crate::sync::sync_cleanup_fs_local_dir_completed;
//use crate::environment::SkyBoxEDF;
use raster::EDF;

pub mod alloc;
pub mod args;
pub mod config;
pub mod container;
pub mod dispatch;
pub mod environment;
pub mod podman;
pub mod slurmd;
pub mod slurmstepd;
pub mod srun;
pub mod sync;

//pub(crate) const SLURM_BATCH_SCRIPT: u32 = 0xfffffffb;

SPANK_PLUGIN!(b"skybox", SLURM_VERSION_NUMBER, SpankSkyBox);

#[derive(Serialize, Default)]
struct SpankSkyBox {
    /*
    container_image: Option<String>,
    container_mounts: Option<SarusMounts>,
    container_workdir: Option<String>,
    container_name: Option<String>,
    container_name_flags: Option<String>,
    container_save: Option<String>,
    container_mount_home: Option<bool>,
    container_remap_root: Option<bool>,
    container_entrypoint: Option<bool>,
    container_entrypoint_log: Option<bool>,
    container_writable: Option<bool>,
    container_env: Option<HashMap<String, String>>,
    environment: Option<String>,
    dump_environment: Option<bool>,
    enabled: bool,
    */
    args: SkyBoxArgs,
    config: SkyBoxConfig,
    edf: Option<EDF>,
    job: Option<Job>,
    run: Option<Run>,
}

#[derive(Clone, Serialize, Default)]
struct Job {
    uid: uid_t,
    gid: gid_t,
    jobid: u32,
    stepid: u32,
    local_task_id: u32,
    global_task_id: u32,
    nodeid: u32,
    local_task_count: u32,
    total_task_count: u32,
    cwd: String,
}

#[derive(Clone, Serialize, Default)]
struct Run {
    name: String,
    pid: u64,
    podman_tmp_path: String,
    syncfile_path: String,
}

pub(crate) fn get_plugin_name() -> String {
    return String::from("skybox");
}

pub(crate) fn plugin_string(s: &str) -> String {
    return format!("[{}] {}", get_plugin_name(), s);
}

pub(crate) fn plugin_err(s: &str) -> Result<(), Box<dyn Error>> {
    return Err(plugin_string(s).into());
}

pub(crate) fn spank_getenv(spank: &mut SpankHandle, var: &str) -> String {
    match spank.getenv(var) {
        Ok(r) => match r {
            Some(v) => v,
            None => String::from(""),
        },
        Err(_) => String::from(""),
    }
}

pub(crate) fn is_skybox_enabled(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> bool {
    if !ssb.config.enabled {
        return false;
    }

    if ssb.edf.is_none() || ssb.edf.clone().unwrap().image == "" {
        return false;
    }

    true
}

pub(crate) fn job_get_info(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let cwd = spank_getenv(spank, "PWD");

    if cwd == "" {
        return plugin_err("couldn't get job cwd path");
    }

    ssb.job = Some(Job {
        uid: spank.job_uid()?,
        gid: spank.job_gid()?,
        jobid: spank.job_id()?,
        stepid: spank.job_stepid()?,
        local_task_id: u32::MAX,
        global_task_id: u32::MAX,
        nodeid: spank.job_nodeid()?,
        local_task_count: spank.job_local_task_count()?,
        total_task_count: spank.job_total_task_count()?,
        cwd: cwd,
    });

    Ok(())
}

pub(crate) fn task_set_info(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let taskid = spank.task_id()?;
    let local_task_id = taskid as u32;
    let global_task_id = spank.local_to_global_id(local_task_id)?;
    let job = &mut ssb.job;
    match job {
        Some(j) => {
            j.local_task_id = local_task_id;
            j.global_task_id = global_task_id;
        }
        None => {
            return plugin_err("couldn't find job structure");
        }
    }
    Ok(())
}
pub(crate) fn run_set_info(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let config = ssb.config.clone();
    let job = ssb.job.clone().unwrap();
    let edf = ssb.edf.clone().unwrap();
    let name = format!("skybox_{}.{}", job.jobid, job.stepid);
    let podman_tmp_path = format!("{}/{}", config.podman_tmp_path, name);
    let syncfile_path = format!("{}/.{}_import.done", edf.parallax_imagestore, name);

    ssb.run = Some(Run {
        name: name,
        pid: u64::MAX,
        podman_tmp_path: podman_tmp_path,
        syncfile_path: syncfile_path,
    });

    Ok(())
}

pub(crate) fn setup_privileged_folders(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let job = ssb.job.clone().unwrap();
    let dir_path = format!("/run/user/{}", job.uid);
    if !Path::new(&dir_path).exists() {
        std::fs::create_dir_all(&dir_path)?;
        let dir_mode = 0o700;
        let dir_perms = Permissions::from_mode(dir_mode);
        std::fs::set_permissions(&dir_path, dir_perms)?;
        let user = match users::get_user_by_uid(job.uid) {
            Some(u) => u,
            None => {
                return plugin_err("couldn't find user");
            }
        };
        let gid = user.primary_group_id();
        chown(dir_path, Some(job.uid), Some(gid))?;
    }
    Ok(())
}

pub(crate) fn setup_folders(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let base_path = match ssb.run.clone() {
        Some(r) => r.podman_tmp_path,
        None => {
            return plugin_err("couldn't find podman_tmp_path");
        }
    };

    let dir_mode = 0o700;
    let mut dir_path;

    dir_path = format!("{}", base_path);
    create_folder(dir_path, dir_mode)?;

    dir_path = format!("{}/graphroot", base_path);
    create_folder(dir_path, dir_mode)?;

    dir_path = format!("{}/runroot", base_path);
    create_folder(dir_path, dir_mode)?;

    Ok(())
}

fn create_folder(path: String, mode: u32) -> Result<(), Box<dyn Error>> {
    if !Path::new(&path).exists() {
        std::fs::create_dir_all(&path)?;
        let perms = Permissions::from_mode(mode);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

pub(crate) fn cleanup_fs_local(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let base_path = match ssb.run.clone() {
        Some(r) => r.podman_tmp_path,
        None => {
            return plugin_err("couldn't find podman_tmp_path");
        }
    };

    if !Path::new(&base_path).exists() {
        ()
    } else {
        match std::fs::remove_dir_all(&base_path) {
            Ok(_) => (),
            Err(e) => {
                let msg = format!("couldn't cleanup \"{:#?}\", error {}", &base_path, e);
                return plugin_err(&msg);
            }
        };
    }
    Ok(())
}

pub(crate) fn cleanup_fs_shared_once(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !is_node_0(ssb, spank) {
        return Ok(());
    }

    let syncfile_path = match ssb.run.clone() {
        Some(r) => r.syncfile_path,
        None => {
            return plugin_err("couldn't find syncfile_path");
        }
    };

    match std::fs::remove_file(&syncfile_path) {
        Ok(_) => (),
        Err(e) => {
            let msg = format!(
                "couldn't cleanup syncfile_path \"{:#?}\", error {}",
                &syncfile_path, e
            );
            return plugin_err(&msg);
        }
    }

    Ok(())
}

pub(crate) fn is_local_task_0(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> bool {
    let job = match ssb.job.clone() {
        Some(j) => j,
        None => {
            return false;
        }
    };

    if job.local_task_id == 0 {
        return true;
    }

    return false;
}

pub(crate) fn is_global_task_0(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> bool {
    let job = match ssb.job.clone() {
        Some(j) => j,
        None => {
            return false;
        }
    };

    if job.global_task_id == 0 {
        return true;
    }

    return false;
}

pub(crate) fn is_node_0(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> bool {
    let job = match ssb.job.clone() {
        Some(j) => j,
        None => {
            return false;
        }
    };

    if job.nodeid == 0 {
        return true;
    }

    return false;
}

pub(crate) fn get_job_env(spank: &mut SpankHandle) -> HashMap<String, String> {
    let mut user_env = HashMap::new();

    let jobenv = match spank.job_env() {
        Ok(j) => j,
        Err(_) => {
            return user_env;
        }
    };

    for e in jobenv.iter() {
        let mut split = e.split("=");

        let size = split.clone().count();
        if size < 2 || size > 3 {
            continue;
        }

        let k = split.next().unwrap();
        let v = split.next().unwrap();
        user_env.insert(String::from(k), String::from(v));
    }

    user_env
}

pub(crate) fn remote_unset_env_vars(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let edf_env = ssb.edf.clone().unwrap().env;
    let mut unset_keys = vec![];

    for (key, value) in edf_env.iter() {
        if value == "" {
            unset_keys.push(key);

            match spank.getenv(key) {
                Ok(opt) => match opt {
                    Some(_) => (),
                    None => continue,
                },
                Err(e) => {
                    let msg = plugin_string(format!("failed to unset {key}: {e}").as_str());
                    spank_log_error!("{msg}");
                    return Err(Box::new(e));
                }
            }

            match spank.unsetenv(key) {
                Ok(_) => {}
                Err(e) => {
                    let msg = plugin_string(format!("failed to unset {key}: {e}").as_str());
                    spank_log_error!("{msg}");
                    return Err(Box::new(e));
                }
            }
        }
    }

    Ok(())
}
