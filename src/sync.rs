use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use slurm_spank::SpankHandle;

use crate::{
    SpankSkyBox, create_folder, dynconf::DynConf, dynconf::apply_dynconf,
    dynconf::load_dynconf_url, get_job_env, get_local_task_id, get_plugin_name, plugin_err,
    plugin_string, podman::podman_pull, podman::podman_start, podman::podman_stop,
    skybox_log_debug, skybox_log_error,
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

    loop {
        let result = std::fs::read_to_string(&pidfile);
        match result {
            Ok(s) => {
                strpid = s;
                break;
            }
            Err(e) => {
                skybox_log_debug!("couldn't read pidfile yet: {e}, wait 1 sec and retry");

                let pause = std::time::Duration::new(1, 0);
                std::thread::sleep(pause);
            }
        }
    }

    let pid: u64 = strpid.parse()?;

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

pub(crate) fn sync_load_dynconf(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) {
    let job_node_id = match spank.job_nodeid() {
        Ok(id) => id,
        Err(e) => {
            skybox_log_error!("cannot collect node_id: {e}");
            return;
        }
    };

    if job_node_id == 0 {
        sync_load_dynconf_query(ssb, spank);
    } else {
        sync_load_dynconf_wait(ssb, spank);
    }
}

fn sync_load_dynconf_query(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) {
    let dynconf = load_dynconf_url(ssb, spank);
    let filepath = get_dynconf_filepath(ssb, spank);

    let mut file = match File::create(filepath) {
        Ok(f) => f,
        Err(_) => {
            return;
        }
    };

    let strjson = match serde_json::to_string(&dynconf) {
        Ok(k) => k,
        Err(_) => String::from("{}"),
    };

    match write!(file, "{}\n", strjson) {
        Ok(_) => (),
        Err(_) => {
            return;
        }
    }
}

fn get_dynconf_filepath(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> String {
    let job_id = match spank.job_id() {
        Ok(id) => id,
        Err(e) => {
            skybox_log_error!("cannot collect job_id: {e}");
            return String::from("");
        }
    };

    let ue = &Some(get_job_env(spank));

    let path = match raster::expand_vars_string(ssb.config.dynconf_path.clone(), ue) {
        Ok(p) => p,
        Err(e) => {
            skybox_log_error!("error in variable expansion: {e}");
            return String::from("");
        }
    };

    let _ = create_folder(path.clone(), 0o700);

    let filename = format!("{}_{}_{}.json", get_plugin_name(), job_id, "dynconf");
    let filepath = format!("{}/{}", path, filename);

    return filepath;
}

fn sync_load_dynconf_wait(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) {
    let filepath = get_dynconf_filepath(ssb, spank);

    let mut loop_num = 0;
    let mut response = String::from("{}");

    loop {
        let result = std::fs::read_to_string(&filepath);
        match result {
            Ok(s) => {
                if s != "" {
                    response = s;
                    break;
                } else {
                    skybox_log_debug!(
                        "couldn't read dynconf at {filepath} yet: EMPTY, wait 1 sec and retry"
                    );

                    let pause = std::time::Duration::new(1, 0);
                    std::thread::sleep(pause);
                }
            }
            Err(e) => {
                skybox_log_debug!(
                    "couldn't read dynconf at {filepath} yet: {e}, wait 1 sec and retry"
                );

                let pause = std::time::Duration::new(1, 0);
                std::thread::sleep(pause);
            }
        }
        loop_num += 1;
        if loop_num >= 10 {
            break;
        }
    }

    let dynconf: DynConf = match serde_json::from_str(&response) {
        Ok(ok) => ok,
        Err(_) => {
            return;
        }
    };

    apply_dynconf(ssb, dynconf);
}

pub(crate) fn sync_cleanup_dynconf(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let filepath = get_dynconf_filepath(ssb, spank);
    skybox_log_debug!("delete {}", &filepath);

    match std::fs::remove_file(&filepath) {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("couldn't cleanup dynconf \"{:#?}\", error {}", &filepath, e);
            return plugin_err(&msg);
        }
    }

    //remove dir if not empty
    let dirpath = match Path::new(&filepath).parent() {
        Some(s) => s,
        None => {
            return Ok(());
        }
    };
    let _ = std::fs::remove_dir(dirpath);

    Ok(())
}
