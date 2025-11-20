use std::error::Error;
use std::path::PathBuf;

use slurm_spank::SpankHandle;

use sarus_suite_podman_driver::{self as pmd, ContainerCtx, ExecutedCommand, PodmanCtx};

use crate::{SpankSkyBox, plugin_err, skybox_log_debug};

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

    if !pmd_image_exists(&edf.image, &ro_ctx) {
        skybox_log_debug!(
            "pulling image \"{}\" from remote in local graphroot",
            edf.image
        );
        pmd_pull(&edf.image, &local_ctx);

        if !pmd_image_exists(&edf.image, &local_ctx) {
            return plugin_err("couldn't find image locally after pull");
        }

        skybox_log_debug!("migrating image \"{}\" to shared imagestore", edf.image);
        pmd_parallax_migrate(&edf.parallax_path, &migrate_ctx, &edf.image)?;

        skybox_log_debug!("removing image \"{}\" from local graphroot", edf.image);
        pmd_rmi(&edf.image, &local_ctx);

        if !pmd_image_exists(&edf.image, &ro_ctx) {
            return plugin_err("couldn't find image on shared imagestore after migration");
        }
    }

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

    return pmd_run(&edf, &run_ctx, &c_ctx, command);
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

    let mut kill = std::process::Command::new("kill")
        .args(["-s", "SIGTERM", &pid.to_string()])
        .spawn()?;
    kill.wait()?;

    Ok(())
}

pub(crate) fn pmd_image_exists(image: &str, ctx: &PodmanCtx) -> bool {
    let prefix = "podman image exists";

    let eb = pmd::image_exists_eb(&image, Some(&ctx));

    log_ec(eb.ec, prefix);

    eb.result
}

pub(crate) fn pmd_pull(image: &str, ctx: &PodmanCtx) -> () {
    let prefix = "podman pull";

    let ec = pmd::pull_ec(&image, Some(&ctx));

    log_ec(ec, prefix);
}

pub(crate) fn pmd_parallax_migrate(
    parallax_path: &str,
    ctx: &PodmanCtx,
    image: &str,
) -> Result<(), Box<dyn Error>> {
    let prefix = "parallax_migrate";

    let ec = pmd::parallax_migrate_ec(&PathBuf::from(parallax_path), ctx, image)?;

    log_ec(ec, prefix);

    Ok(())
}

pub(crate) fn pmd_rmi(image: &str, ctx: &PodmanCtx) -> () {
    let prefix = "podman rmi";

    let ec = pmd::rmi_ec(&image, Some(&ctx));

    log_ec(ec, prefix);
}

pub(crate) fn pmd_run<I, S>(
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
        let lines = stdout.split("\n");
        for line in lines {
            skybox_log_debug!("{prefix} stdout: {}", line);
        }
    }

    if stderr != "" {
        let lines = stderr.split("\n");
        for line in lines {
            skybox_log_debug!("{prefix} stderr: {}", line);
        }
    }
}
