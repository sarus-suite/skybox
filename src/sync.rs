use std::error::Error;
//use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;

use slurm_spank::SpankHandle;

use crate::{
    SpankSkyBox, get_local_task_id, plugin_err, podman::podman_pull,
    podman::podman_start, podman::podman_stop, skybox_log_debug, skybox_log_error,
    TaskInitStatus, SharedMemory
};

pub(crate) fn sync_podman_pull(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let mut shm_arc_clone = Arc::clone(&ssb.shm);
    let shm: &mut SharedMemory = Arc::get_mut(&mut shm_arc_clone).unwrap();

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
                let _ = shm.init_complete.notify_all()?;
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
    let mut shm_arc_clone = Arc::clone(&ssb.shm);
    let shm: &mut SharedMemory = Arc::get_mut(&mut shm_arc_clone).unwrap();

    let exec_task_id = match shm.init_status.get().clone() {
        TaskInitStatus::Exec(task_id) => Some(task_id),
        TaskInitStatus::Done(_) => None, 
        TaskInitStatus::None => unreachable!(),
    };

    if exec_task_id.is_none() || exec_task_id.unwrap() == get_local_task_id(ssb) {
        match podman_start(ssb, spank) {
            Ok(_) => {
                *shm.init_status.get_mut() = TaskInitStatus::Done(true);
                let _ = shm.init_complete.notify_all();
            }
            Err(e) => {
                skybox_log_error!("{e}");
                skybox_log_debug!(
                    "task {} - container start failed",
                    get_local_task_id(ssb)
                );
                *shm.init_status.get_mut() = TaskInitStatus::Done(false);
                let _ = shm.init_complete.notify_all()?;
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
    let mut shm_arc_clone = Arc::clone(&ssb.shm);
    let shm: &mut SharedMemory = Arc::get_mut(&mut shm_arc_clone).unwrap();

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
    let job = ssb.job.clone().unwrap();
    let task_count = job.local_task_count;

    let mut shm_arc_clone = Arc::clone(&ssb.shm);
    let shm: &mut SharedMemory = Arc::get_mut(&mut shm_arc_clone).unwrap();

    // Increase and check 'stop_tasks' counter 
    let prev_stop_tasks;
    match shm.stop_tasks_mutex.lock() {
        Ok(_) => {
            prev_stop_tasks = *shm.stop_tasks.get();
            *shm.stop_tasks.get_mut() += 1;
        },
        Err(e) => {
            skybox_log_debug!(
                "task {} - cannot acquire mutex for stop_tasks: {}",
                get_local_task_id(ssb), e
            );
            return plugin_err("cannot acquire mutex for stop_tasks");
        },
    };

    // Last task kills the container
    if prev_stop_tasks == task_count - 1 {
        sync_cleanup_fs_local_dir_completed(ssb, spank)?;
        podman_stop(ssb, spank)?;
    }

    Ok(())
}

// To chesim: deprecated?
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
