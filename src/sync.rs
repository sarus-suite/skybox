use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use sysinfo::{Pid, ProcessStatus, System};

use slurm_spank::SpankHandle;

use crate::{
    SpankSkyBox, get_local_task_id, plugin_err, plugin_string, podman::podman_pull,
    podman::podman_start, podman::podman_stop, skybox_log_debug, skybox_log_error,
    tracking::track_usage,
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

fn is_process_stopped(pid: usize) -> Result<bool, Box<dyn Error>> {
    let p = Pid::from(pid);

    let s = System::new_all();
    let Some(process) = s.process(p) else {
        return Err(format!("cannot find process {pid}").into());
    };
    let state = process.status();

    if state == ProcessStatus::Stop {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

pub(crate) fn sync_podman_pull(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if is_global_task_0(ssb, spank) {
        match podman_pull(ssb, spank) {
            Ok(_) => {
                sync_podman_pull_done(ssb, spank, 0)?;
            }
            Err(e) => {
                skybox_log_error!("{e}");
                sync_podman_pull_done(ssb, spank, -1)?;
            }
        }
    } else {
        sync_podman_pull_wait(ssb, spank)?;
    }

    Ok(())
}

pub(crate) fn sync_podman_pull_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    skybox_log_debug!(
        "task {} - waiting on image importer",
        get_local_task_id(ssb)
    );

    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        }
    };

    let file_path = run.syncfile_path.clone();
    let pause = std::time::Duration::new(1, 0);
    while std::fs::metadata(&file_path).is_err() {
        std::thread::sleep(pause);
    }

    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim_end();
    let result = line.parse::<i32>().unwrap();

    let msg2 = plugin_string(format!("image importer exited with {}", result).as_str());
    skybox_log_debug!(
        "task {} - image importer exited with {}",
        get_local_task_id(ssb),
        result
    );

    if result != 0 {
        return plugin_err(&msg2);
    }

    Ok(())
}

pub(crate) fn sync_podman_pull_done(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
    result: i32,
) -> Result<(), Box<dyn Error>> {
    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        }
    };
    skybox_log_debug!(
        "task {} - image importer completed with {} - communicating",
        get_local_task_id(ssb),
        result
    );

    let mut file = File::create(run.syncfile_path)?;
    write!(file, "{}\n", result)?;

    if result != 0 {
        let err_msg = format!("podman pull error RC:{}", result);
        return plugin_err(&err_msg);
    } else {
        return Ok(());
    }
}

pub(crate) fn sync_podman_start(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if is_local_task_0(ssb, spank) {
        podman_start(ssb, spank)?;
    }
    sync_podman_start_wait(ssb, spank)?;

    Ok(())
}

pub(crate) fn sync_podman_start_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        }
    };

    let pidfile = format!("{}/pidfile", run.podman_tmp_path);
    let strpid;

    let mut attempts: u32 = 0;
    let pause = std::time::Duration::from_millis(100);
    // Wait max 1 minute for pidfile
    let mut max_attempts: u32 = 600;

    loop {
        let result = std::fs::read_to_string(&pidfile);
        match result {
            Ok(s) => {
                strpid = s;
                break;
            }
            Err(_) => {
                attempts += 1;

                // Fail with error after max attempts
                if attempts >= max_attempts {
                    let msg = format!("failed to read container pidfile {pidfile}.");
                    skybox_log_error!("task {} - {msg}", get_local_task_id(ssb));
                    return plugin_err(&msg);
                }

                // Log first and every 50 retries to limit log spam
                if attempts == 1 || attempts % 50 == 0 {
                    skybox_log_debug!(
                        "task {} - cannot read container pidfile {pidfile} yet, waiting and retrying",
                        get_local_task_id(ssb)
                    );
                }

                std::thread::sleep(pause);
            }
        }
    }

    let pid: usize = strpid.parse()?;
    attempts = 0;
    // Wait max 5 minutes for entrypoint
    max_attempts = 5 * 600;

    loop {
        if is_process_stopped(pid)? {
            break;
        } else {
            attempts += 1;

            // Fail with error after max attempts
            if attempts >= max_attempts {
                let msg = format!("container entrypoint process {strpid} did not complete.");
                skybox_log_error!("task {} - {msg}", get_local_task_id(ssb));
                return plugin_err(&msg);
            }

            // Log first and every 50 retries to limit log spam
            if attempts == 1 || attempts % 50 == 0 {
                skybox_log_debug!(
                    "task {} - container entrypoint process {strpid} hasn't completed yet, waiting and retrying",
                    get_local_task_id(ssb)
                );
            }

            std::thread::sleep(pause);
        }
    }

    let mut newrun = ssb.run.clone().unwrap();
    newrun.pid = pid;

    ssb.run = Some(newrun);

    return Ok(());
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
    /*
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
    */
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
        }
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
    }

    Ok(())
}

pub(crate) fn sync_tracking(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if is_global_task_0(ssb, spank) {
        track_usage(ssb, spank)?;
    }
    Ok(())
}
