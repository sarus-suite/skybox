use std::error::Error;
use std::fs::File;
//use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;

use slurm_spank::SpankHandle;

use crate::{
    SpankSkyBox, get_local_task_id, plugin_err, podman::podman_pull,
    podman::podman_start, podman::podman_stop, skybox_log_debug, skybox_log_error,
    TaskInitStatus, SharedMemory
};

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

pub(crate) fn sync_podman_pull(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let shm: &mut SharedMemory = match ssb.shm {
        Some(s) => Arc::get_mut(&mut s),
        None => return plugin_err("no shm found"),
    };

    // Check and update 'init_status' atomically.
    // If the atomic operation cannot be done for some reason, 
    // gracefully assume every task is an "init task" (TaskInitStatus::None). 
    let mut prev_init_status = TaskInitStatus::None;

    if shm.init_status_mutex.lock().is_ok() {
        prev_init_status = shm.init_status.get().clone();
        if prev_init_status == TaskInitStatus::None {
            *shm.init_status.get_mut() = TaskInitStatus::Exec(get_local_task_id(ssb));
        }
        shm.init_status_mutex.unlock()?;
    }

    // If initialization is not done yet (TaskInitStatus::None),
    // pull the image and start a container.
    // The rest of the tasks will wait until they are finished.
    if prev_init_status == TaskInitStatus::None {
        match podman_pull(ssb, spank) {
            Ok(_) => {
                skybox_log_debug!(
                    "task {} - image import completed",
                    get_local_task_id(ssb)
                );
            },
            Err(e) => {
                skybox_log_error!("{e}");
                skybox_log_debug!(
                    "task {} - image import failed",
                    get_local_task_id(ssb)
                );
                *shm.init_status.get_mut() = TaskInitStatus::Done(false);
                shm.init_complete.notify_all()?;
                return plugin_err("image import failed");
            },
        };
    }

    Ok(())
}

pub(crate) fn sync_podman_start(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let shm: &mut SharedMemory = &mut ssb.shm.unwrap();

    let exec_task_id = match shm.init_status.get().clone() {
        TaskInitStatus::Exec(task_id) => Some(task_id),
        TaskInitStatus::Done(_) => None, 
        TaskInitStatus::None => unreachable!(),
    };

    if exec_task_id.is_none() || exec_task_id.unwrap() == get_local_task_id(ssb) {
        match podman_start(ssb, spank) {
            Ok(_) => {
                *shm.init_status.get_mut() = TaskInitStatus::Done(true);
                shm.init_complete.notify_all();
            }
            Err(e) => {
                skybox_log_error!("{e}");
                skybox_log_debug!(
                    "task {} - container start failed",
                    get_local_task_id(ssb)
                );
                *shm.init_status.get_mut() = TaskInitStatus::Done(false);
                shm.init_complete.notify_all()?;
                return plugin_err("container start failed");
            }
        };
    } else {
        sync_podman_start_wait(ssb, spank)? 
    }

    Ok(())
}

pub(crate) fn sync_podman_start_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let shm: &mut SharedMemory = match ssb.shm {
        Some(s) => Arc::get_mut(&mut s).expect(),
        None => return plugin_err("no shm found"),
    };

    shm.init_complete.wait(&mut shm.init_complete_mutex)?;

    match *shm.init_status.get() {
        TaskInitStatus::Done(false) => {
            skybox_log_debug!(
                "task {} - container start failed",
                get_local_task_id(ssb)
            );
            return plugin_err("container start failed");
        }
        TaskInitStatus::Done(true) => (),
        _ => unreachable!(),
    };

    let run = match &ssb.run {
        Some(o) => o,
        None => return plugin_err("cannot find run struct"),
    };

    let pidfile = format!("{}/pidfile", run.podman_tmp_path);
    let result = std::fs::read_to_string(&pidfile);
    let pid = match result {
        Ok(s) => s.parse()?,
        Err(e) => {
            skybox_log_debug!(
                "task {} - cannot read pidfile: {}",
                get_local_task_id(ssb), e
            );
            return plugin_err("cannot read pidfile");
        }
    };

    let mut new_run = ssb.run.clone().unwrap();
    new_run.pid = pid;
    ssb.run = Some(new_run);

    Ok(())
}

pub(crate) fn sync_podman_stop(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let run = ssb.run.clone().unwrap();
    let job = ssb.job.clone().unwrap();
    let task_id = job.local_task_id;
    let task_count = job.local_task_count;

    // create sync folder if doesn't exist
    let completed_dir_path = format!("{}/completed", run.podman_tmp_path);
    if !std::path::Path::new(&completed_dir_path).exists() {
        std::fs::create_dir_all(&completed_dir_path)?;
    }

    // touch file in sync folder
    let completed_file_path = format!("{}/task_{}.exit", completed_dir_path, task_id);
    File::create(completed_file_path)?;

    // Wait for all tasks to stop podman.
    let readdir = std::fs::read_dir(&completed_dir_path)?;
    if (readdir.count() as u32) == task_count {
        sync_cleanup_fs_local_dir_completed(ssb, spank)?;
        podman_stop(ssb, spank)?;
    }

    Ok(())
}

pub(crate) fn sync_cleanup_fs_local_dir_completed(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let base_path = match ssb.run.clone() {
        Some(r) => r.podman_tmp_path,
        None => {
            return plugin_err("couldn't find podman_tmp_path");
        }
    };

    let completed_dir_path = format!("{}/completed", base_path);

    if !Path::new(&completed_dir_path).exists() {
        ()
    } else {
        match std::fs::remove_dir_all(&completed_dir_path) {
            Ok(_) => (),
            Err(e) => {
                let msg = format!(
                    "couldn't cleanup \"{:#?}\", error {}",
                    &completed_dir_path, e
                );
                return plugin_err(&msg);
            }
        };
    }

    Ok(())
}

pub(crate) fn sync_cleanup_fs_shared(
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
        },
    };

    skybox_log_debug!("delete {}", &syncfile_path);
    match std::fs::remove_file(&syncfile_path) {
        Ok(_) => (),
        Err(e) => {
            let msg = format!(
                "couldn't cleanup syncfile_path \"{:#?}\", error {}",
                &syncfile_path, e
            );
            return plugin_err(&msg);
        }
    };

    Ok(())
}
