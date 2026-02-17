use slurm_spank::{Context, SpankHandle};

use std::error::Error;

use raster::mount::SarusMount;

use crate::{SLURM_BATCH_SCRIPT, SpankSkyBox, skybox_log_debug, spank_getenv};

pub(crate) fn load_edf(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let edf_name = match &ssb.args.edf {
        Some(name) => String::from(name),
        None => {
            return Ok(());
        }
    };

    let edf: raster::EDF;

    match spank.context()? {
        Context::Local | Context::Allocator => {
            edf = local_edf_render(edf_name)?;
        },
        Context::Remote => {
            edf = spank_remote_get_edf(spank)?;
        },
        _ => {
            return Ok(());
        }
    }

    ssb.edf = Some(edf);
    Ok(())
}

fn local_edf_render(
    path: String,
) -> Result<raster::EDF, Box<dyn Error>> {
    let edf = raster::render(path)?;
    define_edf_expanded_envvar(&edf)?;
    Ok(edf)
}

fn define_edf_expanded_envvar(
    edf: &raster::EDF,
) -> Result<(), Box<dyn Error>> {
    let key = "SLURM_EDF_EXPANDED";
    let value = edf.to_toml_string()?;
    unsafe {
        std::env::set_var(key, value);
    }
    Ok(())
}

/*
fn spank_remote_edf_render(
    path: String,
    spank: &mut SpankHandle,
) -> Result<raster::EDF, Box<dyn Error>> {
    let sp = spank_remote_get_search_paths(spank);
    let ue = &Some(get_job_env(spank));
    Ok(raster::render_from_search_paths(path, sp, ue)?)
}
*/

fn spank_remote_get_edf(
    spank: &mut SpankHandle,
) -> Result<raster::EDF, Box<dyn Error>> {
    let key = "SLURM_EDF_EXPANDED";
    let value = spank_getenv(spank, key);
    let edf = raster::get_edf_from_string(value)?;
    Ok(edf)
}

/*
fn spank_remote_get_search_paths(spank: &mut SpankHandle) -> Vec<String> {
    let mut search_paths = vec![];

    let user_sp = spank_remote_get_user_search_paths(spank);
    search_paths.extend(user_sp);

    let sys_sp = raster::get_sys_search_paths();
    search_paths.extend(sys_sp);

    search_paths
}


fn spank_remote_get_user_search_paths(spank: &mut SpankHandle) -> Vec<String> {
    let mut search_paths = vec![];

    let mut edf_path = spank_getenv(spank, "EDF_PATH");

    if edf_path == "" {
        let home_path = spank_getenv(spank, "HOME");

        if home_path != "" {
            edf_path = format!("{home_path}/.edf");
        }
    }

    if edf_path != "" {
        search_paths.push(edf_path);
    }

    search_paths
}

pub(crate) fn update_edf_defaults_via_config(ssb: &mut SpankSkyBox) -> Result<(), Box<dyn Error>> {
    let mut edf = match ssb.edf.clone() {
        Some(e) => e,
        None => {
            return Ok(());
        }
    };

    let c = ssb.config.clone();

    if edf.parallax_enable == false {
        edf.parallax_enable = true;
    }

    if edf.parallax_imagestore == "" {
        edf.parallax_imagestore = c.parallax_imagestore;
    }

    if edf.parallax_mount_program == "" {
        edf.parallax_mount_program = c.parallax_mount_program;
    }

    if edf.parallax_path == "parallax" {
        edf.parallax_path = c.parallax_path;
    }

    if edf.podman_module == "hpc" {
        edf.podman_module = c.podman_module;
    }

    if edf.podman_path == "podman" {
        edf.podman_path = c.podman_path;
    }

    if edf.podman_tmp_path == "/dev/shm" {
        edf.podman_tmp_path = c.podman_tmp_path;
    }

    ssb.edf = Some(edf);

    Ok(())
}
*/

pub(crate) fn modify_edf_for_sbatch(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let job = match &ssb.job {
        Some(j) => j,
        None => {
            skybox_log_debug!("cannot find job data at this stage");
            return Ok(());
        }
    };

    let stepid = job.stepid;
    //skybox_log_debug!("STEP: {stepid} vs {SLURM_BATCH_SCRIPT}");
    if stepid == SLURM_BATCH_SCRIPT {
        let mut edf = match ssb.edf.clone() {
            Some(e) => e,
            None => {
                return Ok(());
            }
        };

        let argv = match spank.job_argv() {
            Ok(a) => a,
            Err(_) => {
                skybox_log_debug!("cannot read job args");
                return Ok(());
            }
        };

        let sbatch_script = match argv.get(0) {
            Some(s) => s,
            None => {
                skybox_log_debug!("cannot read job argv[0]");
                return Ok(());
            }
        };

        let flags = String::from("bind,ro,nosuid,nodev,private");
        let mount_string = format!("{}:{}:{}", &sbatch_script, &sbatch_script, &flags);

        let sm = match SarusMount::try_new(mount_string, &None) {
            Ok(ok) => ok,
            Err(_) => {
                skybox_log_debug!("cannot create sbatch script mount defintion");
                return Ok(());
            }
        };

        skybox_log_debug!("NEW MOUNT: {}", sbatch_script);
        edf.mounts.append(&mut vec![sm]);

        ssb.edf = Some(edf);
    }
    Ok(())
}
