use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use slurm_spank::{
    SpankHandle,
    spank_log_debug,
    spank_log_error,
    spank_log_info,
    //spank_log_user,
};

use crate::{SpankSkyBox, is_global_task_0, is_local_task_0, plugin_err, plugin_string};
use sarus_suite_podman_driver::{self as pmd, PodmanCtx};

pub(crate) fn podman_pull_once(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if is_global_task_0(ssb, spank) {
        match podman_pull(ssb, spank) {
            Ok(_) => {
                podman_pull_done(ssb, spank, 0)?;
            }
            Err(e) => {
                spank_log_error!("{}", plugin_string(format!("{}", e).as_str()));
                podman_pull_done(ssb, spank, -1)?;
            }
        }
    } else {
        podman_pull_wait(ssb, spank)?;
    }

    Ok(())
}

pub(crate) fn podman_pull(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find edf");
        }
    };

    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        }
    };

    let graphroot = format!("{}/graphroot", run.podman_tmp_path);
    let runroot = format!("{}/runroot", run.podman_tmp_path);

    let ro_ctx = PodmanCtx {
        podman_path: PathBuf::from(&edf.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: None,
        ro_store: Some(PathBuf::from(&edf.parallax_imagestore)),
    };

    let local_ctx = PodmanCtx {
        podman_path: PathBuf::from(&edf.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: None,
        ro_store: None,
    };

    let migrate_ctx = PodmanCtx {
        podman_path: PathBuf::from(&edf.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: None,
        parallax_mount_program: None,
        ro_store: Some(PathBuf::from(&edf.parallax_imagestore)),
    };

    if !pmd::image_exists(&edf.image, Some(&ro_ctx)) {
        pmd::pull(&edf.image, Some(&local_ctx));

        if !pmd::image_exists(&edf.image, Some(&local_ctx)) {
            return plugin_err("couldn't find image locally after pull");
        }

        pmd::parallax_migrate(&PathBuf::from(&edf.parallax_path), &migrate_ctx, &edf.image)?;
        pmd::rmi(&edf.image, Some(&local_ctx));

        if !pmd::image_exists(&edf.image, Some(&ro_ctx)) {
            return plugin_err("couldn't find image on imagestore after migrate");
        }
    }

    Ok(())
}

pub(crate) fn podman_pull_done(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
    result: i32,
) -> Result<(), Box<dyn Error>> {
    let msg =
        plugin_string(format!("image importer completed with {} - communicating", result).as_str());
    spank_log_info!("{msg}");

    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        }
    };

    let mut file = File::create(run.syncfile_path)?;
    write!(file, "{}\n", result)?;

    if result != 0 {
        return plugin_err("");
    } else {
        return Ok(());
    }
}

pub(crate) fn podman_pull_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let msg1 = plugin_string("waiting on image importer");
    spank_log_info!("{msg1}");

    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        }
    };

    let pause = std::time::Duration::new(1, 0);
    let file_path = run.syncfile_path.clone();
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
    spank_log_info!("{msg2}");

    if result != 0 {
        return plugin_err(&msg2);
    }

    Ok(())
}

pub(crate) fn podman_start_once(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if is_local_task_0(ssb, spank) {
        podman_start(ssb, spank)?;
    }
    podman_start_wait(ssb, spank)?;

    Ok(())
}

pub(crate) fn podman_start(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find edf");
        }
    };

    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        }
    };

    let graphroot = format!("{}/graphroot", run.podman_tmp_path);
    let runroot = format!("{}/runroot", run.podman_tmp_path);
    let pidfile = format!("{}/pidfile", run.podman_tmp_path);
    let command = vec!["sleep", "infinity"];

    let c_ctx = pmd::ContainerCtx {
        name: run.name.clone(),
        interactive: false,
        detach: true,
        set_env: true,
        pidfile: Some(PathBuf::from(pidfile.clone())),
    };

    let run_ctx = PodmanCtx {
        podman_path: PathBuf::from(&edf.podman_path),
        module: Some(String::from(&edf.podman_module)),
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: Some(PathBuf::from(&edf.parallax_mount_program)),
        ro_store: Some(PathBuf::from(&edf.parallax_imagestore)),
    };

    let output = pmd::run_from_edf_output(&edf, Some(&run_ctx), &c_ctx, command);

    match output.status.code() {
        Some(rc) => {
            if rc != 0 {
                return plugin_err(format!("podman run exited with {rc}").as_str());
            }
        }
        None => return plugin_err("podman run failed badly"),
    };

    return Ok(());
}

pub(crate) fn podman_get_pid(ssb: &mut SpankSkyBox) -> Result<u64, Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return Err("couldn't find run".into());
        }
    };

    let pidfile = format!("{}/pidfile", run.podman_tmp_path);
    let strpid = std::fs::read_to_string(&pidfile)?;
    let pid: u64 = strpid.parse()?;
    Ok(pid)
}

pub(crate) fn podman_start_wait(
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
                let msg = plugin_string(
                    format!("couldn't read pidfile yet: {e}, wait 1 sec and retry").as_str(),
                );
                spank_log_debug!("{msg}");

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

pub(crate) fn podman_stop(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //let pid = podman_get_pid(ssb)? as i32;
    let pid = podman_get_pid(ssb)?;

    // Kill it
    //nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), nix::sys::signal::Signal::SIGTERM)?;

    let mut kill = std::process::Command::new("kill")
        .args(["-s", "SIGTERM", &pid.to_string()])
        .spawn()?;
    kill.wait()?;

    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find edf");
        }
    };

    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        }
    };

    let graphroot = format!("{}/graphroot", run.podman_tmp_path);
    let runroot = format!("{}/runroot", run.podman_tmp_path);

    let ro_ctx = PodmanCtx {
        podman_path: PathBuf::from(&edf.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: None,
        ro_store: Some(PathBuf::from(&edf.parallax_imagestore)),
    };

    let name = ssb.run.clone().unwrap().name;

    let output = pmd::inspect(name.as_str(), Some("{{.State.Status}}"), Some(&ro_ctx));
    let status = String::from_utf8(output.stdout)?;
    if status == "" {
        let msg = plugin_string("podman inspect did not produce any usable output");
        spank_log_error!("{msg}");
    } else {
        let msg = plugin_string(format!("container {} status is: {}", name, status).as_str());
        spank_log_info!("{msg}");
    }

    pmd::rm(name.as_str(), Some(&ro_ctx));

    Ok(())
}

pub(crate) fn podman_stop_once(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let run = ssb.run.clone().unwrap();
    let job = ssb.job.clone().unwrap();
    let task_id = job.local_task_id;
    let task_count = job.local_task_count;

    // create folder if doesn't exist
    let completed_dir_path = format!("{}/completed", run.podman_tmp_path);
    if !std::path::Path::new(&completed_dir_path).exists() {
        std::fs::create_dir_all(&completed_dir_path)?;
    }

    // touch file in the folder
    let completed_file_path = format!("{}/task_{}.exit", completed_dir_path, task_id);
    //let mut file = File::create(completed_file_path)?;
    File::create(completed_file_path)?;

    // Last task will wait to stop the container process.
    //spank_log_debug!("STOP? {} vs {}", task_id, task_count);

    // Wait for All tasks to stop podman.
    let readdir = std::fs::read_dir(&completed_dir_path)?;
    if (readdir.count() as u32) == task_count {
        podman_stop(ssb, spank)?;
    }

    Ok(())
}
