use slurm_spank::{Context, SpankHandle};

use std::error::Error;

use crate::{SpankSkyBox, get_job_env, spank_getenv};

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
        Context::Local => {
            edf = raster::render(edf_name)?;
        }
        Context::Remote => {
            edf = spank_remote_edf_render(edf_name, spank)?;
        }
        _ => {
            return Ok(());
        }
    }

    ssb.edf = Some(edf);
    Ok(())
}

fn spank_remote_edf_render(
    path: String,
    spank: &mut SpankHandle,
) -> Result<raster::EDF, Box<dyn Error>> {
    let sp = spank_remote_get_search_paths(spank);
    let ue = &Some(get_job_env(spank));
    Ok(raster::render_from_search_paths(path, sp, ue)?)
}

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
