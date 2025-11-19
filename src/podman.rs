use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use slurm_spank::SpankHandle;

use sarus_suite_podman_driver::{self as pmd, ContainerCtx, ExecutedCommand, PodmanCtx};

use crate::{
    SpankSkyBox,
    //cleanup_fs_local,
    //cleanup_fs_shared_once,
    get_local_task_id,
    is_global_task_0,
    is_local_task_0,
    plugin_err,
    plugin_string,
    skybox_log_debug,
    skybox_log_error,
    //skybox_log_info,
    sync_cleanup_fs_local_dir_completed,
};

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
                //spank_log_error!("{}", plugin_string(format!("{}", e).as_str()));
                skybox_log_error!("{e}");
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

    if !image_exists(&edf.image, &ro_ctx) {
        skybox_log_debug!(
            "pulling image \"{}\" from remote in local graphroot",
            edf.image
        );
        pull(&edf.image, &local_ctx);

        if !image_exists(&edf.image, &local_ctx) {
            return plugin_err("couldn't find image locally after pull");
        }

        skybox_log_debug!("migrating image \"{}\" to shared imagestore", edf.image);
        parallax_migrate(&edf.parallax_path, &migrate_ctx, &edf.image)?;

        skybox_log_debug!("removing image \"{}\" from local graphroot", edf.image);
        rmi(&edf.image, &local_ctx);

        if !image_exists(&edf.image, &ro_ctx) {
            return plugin_err("couldn't find image on shared imagestore after migration");
        }
    }

    Ok(())
}

pub(crate) fn podman_pull_done(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
    result: i32,
) -> Result<(), Box<dyn Error>> {
    //let msg =
    //    plugin_string(format!("image importer completed with {} - communicating", result).as_str());
    //spank_log_info!("{msg}");

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

pub(crate) fn podman_pull_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    //let msg1 = plugin_string("waiting on image importer");
    //spank_log_info!("{msg1}");
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
    //spank_log_info!("{msg2}");
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

    return podman_run(&edf, &run_ctx, &c_ctx, command);
}

pub(crate) fn podman_get_pid_from_file(ssb: &mut SpankSkyBox) -> Result<u64, Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return Err("couldn't find run data".into());
        }
    };

    //Try to read from pidfile
    let pidfile = format!("{}/pidfile", run.podman_tmp_path);
    if std::path::Path::new(&pidfile).exists() {
        let strpid = match std::fs::read_to_string(&pidfile) {
            Ok(s) => s,
            Err(_) => {
                let err_msg = format!("cannot read pid from {pidfile}");
                return Err(err_msg.into());
            }
        };
        let pid: u64 = match strpid.parse() {
            Ok(p) => p,
            Err(_) => {
                let err_msg = format!("cannot convert {strpid} to number");
                return Err(err_msg.into());
            }
        };
        return Ok(pid);
    } else {
        let err_msg = format!("{pidfile} NOT FOUND!");
        Err(err_msg.into())
    }
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
                //let msg = plugin_string(
                //    format!("couldn't read pidfile yet: {e}, wait 1 sec and retry").as_str(),
                //);
                //spank_log_debug!("{msg}");
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

pub(crate) fn podman_stop(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run data");
        }
    };

    let pid = run.pid;

    /*
    let mut check_mount = std::process::Command::new("bash")
        .args(["-c", "mount | grep overlay"])
        .spawn()?;

    let mut check_mount_output = match check_mount.wait_with_output() {
        Ok(out) => out,
        Err(e) => {
            let msg = plugin_string(
                format!("couldn't check mounts before stopping podman container, {e}").as_str(),
            );
            spank_log_debug!("{msg}");

            return Err(msg.into());
        },
    };

    let mut check_mount_rc = match check_mount_output.status.code() {
        Some(ok) => ok,
        None => {
            let msg = plugin_string(
                format!("check_mount_before exited by signal").as_str(),
            );
            spank_log_debug!("{msg}");

            return Err(msg.into());
        },
    };

    let mut check_mount_stdout = String::from_utf8(check_mount_output.stdout)?;
    let mut check_mount_stderr = String::from_utf8(check_mount_output.stderr)?;

    let mut msg_rc = format!("check_mount_before: exit code: {}", check_mount_rc);
    let mut msg_stdout = format!("check_mount_before: stdout: {}", check_mount_stdout);
    let mut msg_stderr = format!("check_mount_before: stderr: {}", check_mount_stderr);

    spank_log_debug!("{}", plugin_string(&msg_rc));
    spank_log_debug!("{}", plugin_string(&msg_stdout));
    spank_log_debug!("{}", plugin_string(&msg_stderr));
    */

    // Kill it
    let mut kill = std::process::Command::new("kill")
        .args(["-s", "SIGTERM", &pid.to_string()])
        .spawn()?;
    kill.wait()?;

    /*
    check_mount = std::process::Command::new("bash")
        .args(["-c", "mount | grep overlay"])
        .spawn()?;
    //check_mount.wait_with_output()?;

    check_mount_output = match check_mount.wait_with_output() {
        Ok(out) => out,
        Err(e) => {
            let msg = plugin_string(
                format!("couldn't check mounts after stopping podman container, {e}").as_str(),
            );
            spank_log_debug!("{msg}");

            return Err(msg.into());
        },
    };

    check_mount_rc = match check_mount_output.status.code() {
        Some(ok) => ok,
        None => {
            let msg = plugin_string(
                format!("check_mount_after exited by signal").as_str(),
            );
            spank_log_debug!("{msg}");

            return Err(msg.into());
        },
    };

    check_mount_stdout = String::from_utf8(check_mount_output.stdout)?;
    check_mount_stderr = String::from_utf8(check_mount_output.stderr)?;

    msg_rc = format!("check_mount_after: exit code: {}", check_mount_rc);
    msg_stdout = format!("check_mount_after: stdout: {}", check_mount_stdout);
    msg_stderr = format!("check_mount_after: stderr: {}", check_mount_stderr);

    spank_log_debug!("{}", plugin_string(&msg_rc));
    spank_log_debug!("{}", plugin_string(&msg_stdout));
    spank_log_debug!("{}", plugin_string(&msg_stderr));
    */
    /*
    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find edf");
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
    */

    // STOP
    /*
    let stop = pmd::stop_output(name.as_str(), Some(&ro_ctx));
    let stop_command = stop.command;
    let stop_rc = match stop.output.status.code() {
        Some(ok) => ok,
        None => {
            let msg = plugin_string(
                format!("podman stop exited by signal").as_str(),
            );
            spank_log_debug!("{msg}");

            return Err(msg.into());
        },
    };

    let mut stop_stdout = String::from_utf8(stop.output.stdout)?;
    if stop_stdout.ends_with("\n") {
        stop_stdout.pop();
    };

    let mut stop_stderr = String::from_utf8(stop.output.stderr)?;
    if stop_stderr.ends_with("\n") {
        stop_stderr.pop();
    };

    let msg_cmd = format!("CMD: {}", stop_command);
    msg_rc = format!("podman stop: exit code: {}", stop_rc);
    msg_stdout = format!("podman stop: stdout: {}", stop_stdout);
    msg_stderr = format!("podman stop: stderr: {}", stop_stderr);

    spank_log_debug!("{}", plugin_string(&msg_cmd));
    spank_log_debug!("{}", plugin_string(&msg_rc));
    spank_log_debug!("{}", plugin_string(&msg_stdout));
    spank_log_debug!("{}", plugin_string(&msg_stderr));
    */
    /*
    if stop_status == "" {
        let msg = plugin_string("podman stop did not produce any usable output");
        spank_log_error!("{msg}");
    } else {
        let msg = plugin_string(format!("container {} status is: {}", name, stop_status).as_str());
        spank_log_info!("{msg}");
    }
    */

    // INSPECT
    /*
    let output = pmd::inspect(name.as_str(), Some("{{.State.Status}}"), Some(&ro_ctx));
    let status = String::from_utf8(output.stdout)?;

    if status == "" {
        let msg = plugin_string("podman inspect did not produce any usable output");
        spank_log_debug!("{msg}");
    } else {
        let msg = plugin_string(format!("container {} status is: {}", name, status).as_str());
        spank_log_info!("{msg}");
    }
    */

    // REMOVE
    /*
    let rm_output = pmd::rm_output(name.as_str(), Some(&ro_ctx));
    let rm_status = String::from_utf8(rm_output.stdout)?;

    if rm_status == "" {
        let msg = plugin_string("podman remove did not produce any usable output");
        spank_log_debug!("{msg}");
    } else {
        let msg = plugin_string(format!("podman rm {} : stdout {}", name, rm_status).as_str());
        spank_log_info!("{msg}");
    }
    */

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

pub(crate) fn image_exists(image: &str, ctx: &PodmanCtx) -> bool {
    let prefix = "podman image exists";

    let eb = pmd::image_exists_eb(&image, Some(&ctx));

    log_ec(eb.ec, prefix);

    eb.result
}

pub(crate) fn pull(image: &str, ctx: &PodmanCtx) -> () {
    let prefix = "podman pull";

    let ec = pmd::pull_ec(&image, Some(&ctx));

    log_ec(ec, prefix);
}

pub(crate) fn parallax_migrate(
    parallax_path: &str,
    ctx: &PodmanCtx,
    image: &str,
) -> Result<(), Box<dyn Error>> {
    let prefix = "parallax_migrate";

    let ec = pmd::parallax_migrate_ec(&PathBuf::from(parallax_path), ctx, image)?;

    log_ec(ec, prefix);

    Ok(())
}

pub(crate) fn rmi(image: &str, ctx: &PodmanCtx) -> () {
    let prefix = "podman rmi";

    let ec = pmd::rmi_ec(&image, Some(&ctx));

    log_ec(ec, prefix);
}

pub(crate) fn podman_run<I, S>(
    edf: &raster::EDF,
    p_ctx: &PodmanCtx,
    c_ctx: &ContainerCtx,
    cmd: I,
) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let prefix = "podman run";

    let ec = pmd::run_from_edf_ec(&edf, Some(&p_ctx), &c_ctx, cmd);

    log_ec(ec.clone(), prefix);

    match ec.output.status.code() {
        Some(rc) => {
            if rc != 0 {
                return plugin_err(format!("podman run exited with {rc}").as_str());
            }
        }
        None => return plugin_err("podman run failed badly"),
    };

    Ok(())
}

pub(crate) fn log_ec(ec: ExecutedCommand, prefix: &str) {
    let rc = match ec.output.status.code() {
        Some(ok) => format!("{ok}"),
        None => {
            skybox_log_debug!("{prefix} exited by signal");
            String::from("UNKNOWN")
        }
    };

    let mut stdout = match String::from_utf8(ec.output.stdout) {
        Ok(ok) => ok,
        Err(_) => String::from(""),
    };
    if stdout.ends_with("\n") {
        stdout.pop();
    };

    let mut stderr = match String::from_utf8(ec.output.stderr) {
        Ok(ok) => ok,
        Err(_) => String::from(""),
    };
    if stderr.ends_with("\n") {
        stderr.pop();
    };

    skybox_log_debug!("CMD: {}", ec.command);
    skybox_log_debug!("{prefix} exit code: {}", rc);

    if stdout != "" {
        skybox_log_debug!("{prefix} stdout: {}", stdout);
    }

    if stderr != "" {
        skybox_log_debug!("{prefix} stderr: {}", stderr);
    }
}
