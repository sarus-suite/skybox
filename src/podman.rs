use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use sysinfo::{Pid, System};

use slurm_spank::{SpankHandle, spank_log_user};

use sarus_suite_podman_driver::{self as pmd, ContainerCtx, PodmanCtx};

use crate::config::setup_imagestore;
use crate::{SpankSkyBox, plugin_err, skybox_log_debug, skybox_log_error};

pub(crate) const PODMAN_PIDFILE_NAME: &str = "pidfile";

fn process_exists(pid: usize) -> bool {
    let p = Pid::from(pid);

    let s = System::new_all();
    let ret = match s.process(p) {
        None => false,
        Some(process) => {
            let state = process.status();
            skybox_log_debug!("process {pid} status is {state}");
            true
        }
    };
    ret
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

    let config = &ssb.config;
    setup_imagestore(config)?;

    let graphroot = format!("{}/graphroot", run.podman_tmp_path);
    let runroot = format!("{}/runroot", run.podman_tmp_path);

    let ro_ctx = PodmanCtx {
        podman_path: PathBuf::from(&config.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: None,
        ro_store: Some(PathBuf::from(&config.parallax_imagestore)),
        podman_env: None,
    }
    .with_env("PARALLAX_MP_UID", config.parallax_mp_uid.to_string())
    .with_env("PARALLAX_MP_GID", config.parallax_mp_gid.to_string())
    .with_env(
        "PARALLAX_MP_SQUASHFUSE_CMD",
        config.parallax_mp_squashfuse_path.clone(),
    )
    .with_env("PARALLAX_MP_LOGFILE", config.parallax_mp_logfile.clone());

    let local_ctx = PodmanCtx {
        podman_path: PathBuf::from(&config.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: None,
        ro_store: None,
        podman_env: None,
    }
    .with_env("PARALLAX_MP_UID", config.parallax_mp_uid.to_string())
    .with_env("PARALLAX_MP_GID", config.parallax_mp_gid.to_string())
    .with_env(
        "PARALLAX_MP_SQUASHFUSE_CMD",
        config.parallax_mp_squashfuse_path.clone(),
    )
    .with_env("PARALLAX_MP_LOGFILE", config.parallax_mp_logfile.clone());

    let migrate_ctx = PodmanCtx {
        podman_path: PathBuf::from(&config.podman_path),
        module: None,
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: None,
        parallax_mount_program: None,
        ro_store: Some(PathBuf::from(&config.parallax_imagestore)),
        podman_env: None,
    }
    .with_env("PARALLAX_MP_UID", config.parallax_mp_uid.to_string())
    .with_env("PARALLAX_MP_GID", config.parallax_mp_gid.to_string())
    .with_env(
        "PARALLAX_MP_SQUASHFUSE_CMD",
        config.parallax_mp_squashfuse_path.clone(),
    )
    .with_env("PARALLAX_MP_LOGFILE", config.parallax_mp_logfile.clone());

    if !pmd_image_exists(&edf.image, &ro_ctx)? {
        skybox_log_debug!(
            "pulling image \"{}\" from remote in local graphroot",
            edf.image
        );
        pmd_pull(&edf.image, &local_ctx)?;

        if !pmd_image_exists(&edf.image, &local_ctx)? {
            return plugin_err("podman pull failed, cannot find image in local graphroot");
        }

        skybox_log_debug!("migrating image \"{}\" to shared imagestore", edf.image);
        pmd_parallax_migrate(&config.parallax_path, &migrate_ctx, &edf.image)?;

        skybox_log_debug!("removing image \"{}\" from local graphroot", edf.image);
        if let Err(error) = pmd_rmi(&edf.image, &local_ctx) {
            skybox_log_error!(
                "failed to remove image \"{}\" from local graphroot: {}",
                edf.image,
                error
            );
        }

        if !pmd_image_exists(&edf.image, &ro_ctx)? {
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

    let job = match &ssb.job {
        Some(job) => job,
        None => {
            return plugin_err("couldn't find job");
        }
    };

    let config = &ssb.config;

    let graphroot = format!("{}/graphroot", run.podman_tmp_path);
    let runroot = format!("{}/runroot", run.podman_tmp_path);
    let pidfile = format!("{}/{}", run.podman_tmp_path, PODMAN_PIDFILE_NAME);
    //let command = vec!["sleep", "infinity"];
    let command = vec!["sh", "-c", "kill -STOP $$ ; exit 0"];
    //let command = vec!["sh", "-l", "-c", "exec sh -c 'kill -STOP $$ ; exit 0'"];

    let c_ctx = ContainerCtx {
        name: run.name.clone(),
        interactive: false,
        tty: false,
        detach: true,
        auto_remove: true,
        set_env: false,
        pidfile: Some(PathBuf::from(pidfile.clone())),
        user: Some(job.uid.to_string()),
    };

    let run_ctx = PodmanCtx {
        podman_path: PathBuf::from(&config.podman_path),
        module: Some(String::from(&config.podman_module)),
        graphroot: Some(PathBuf::from(&graphroot)),
        runroot: Some(PathBuf::from(&runroot)),
        parallax_mount_program: Some(PathBuf::from(&config.parallax_mount_program)),
        ro_store: Some(PathBuf::from(&config.parallax_imagestore)),
        podman_env: None,
    }
    .with_env("PARALLAX_MP_UID", config.parallax_mp_uid.to_string())
    .with_env("PARALLAX_MP_GID", config.parallax_mp_gid.to_string())
    .with_env(
        "PARALLAX_MP_SQUASHFUSE_CMD",
        config.parallax_mp_squashfuse_path.clone(),
    )
    .with_env("PARALLAX_MP_LOGFILE", config.parallax_mp_logfile.clone());

    skybox_log_debug!(
        "mount env: PARALLAX_MP_UID={} PARALLAX_MP_GID={}",
        config.parallax_mp_uid.to_string(),
        config.parallax_mp_gid.to_string()
    );

    return pmd_run(&edf, &config, &run_ctx, &c_ctx, command);
}

// TODO: clarify usefulness of this function since the pid is already acquired in crate::sync::sync_podman_start_wait()
pub(crate) fn podman_get_pid_from_file(ssb: &mut SpankSkyBox) -> Result<usize, Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return Err("couldn't find run data".into());
        }
    };

    //Try to read from pidfile
    let pidfile = format!("{}/{}", run.podman_tmp_path, PODMAN_PIDFILE_NAME);
    if std::path::Path::new(&pidfile).exists() {
        let strpid = match std::fs::read_to_string(&pidfile) {
            Ok(s) => s,
            Err(_) => {
                let err_msg = format!("cannot read pid from {pidfile}");
                return Err(err_msg.into());
            }
        };
        let pid: usize = match strpid.parse() {
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

    skybox_log_debug!("stopping container, process {pid}");
    let mut kill = std::process::Command::new("kill")
        .args(["-s", "SIGCONT", &pid.to_string()])
        .spawn()?;
    kill.wait()?;

    if process_exists(pid) {
        skybox_log_debug!("process {pid} is still there, waiting one more second.");
        let pause = std::time::Duration::from_secs(1);
        std::thread::sleep(pause);
    }

    if process_exists(pid) {
        skybox_log_debug!("process {pid} is still there, terminating it.");
        let mut kill = std::process::Command::new("kill")
            .args(["-s", "SIGTERM", &pid.to_string()])
            .spawn()?;
        kill.wait()?;
    }

    Ok(())
}

pub(crate) fn pmd_image_exists(image: &str, ctx: &PodmanCtx) -> pmd::Result<bool> {
    pmd::image_exists(image, Some(ctx))
}

pub(crate) fn pmd_pull(image: &str, ctx: &PodmanCtx) -> pmd::Result<()> {
    pmd::pull(image, Some(ctx))
}

pub(crate) fn pmd_parallax_migrate(
    parallax_path: &str,
    ctx: &PodmanCtx,
    image: &str,
) -> Result<(), Box<dyn Error>> {
    pmd::parallax_migrate(&PathBuf::from(parallax_path), ctx, image)?;
    Ok(())
}

pub(crate) fn pmd_rmi(image: &str, ctx: &PodmanCtx) -> pmd::Result<()> {
    pmd::rmi(image, Some(ctx))
}

pub(crate) fn pmd_run<I, S>(
    edf: &raster::EDF,
    config: &raster::Config,
    p_ctx: &PodmanCtx,
    c_ctx: &ContainerCtx,
    cmd: I,
) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let t0 = Instant::now();
    let result = pmd::run_from_edf_output(edf, Some(p_ctx), c_ctx, cmd);
    let tend = t0.elapsed();

    if config.perfmon {
        spank_log_user!(
            "skybox-perf: Podman run elapsed time: {:.6} sec",
            tend.as_secs_f64()
        );
    }

    result?;
    Ok(())
}
