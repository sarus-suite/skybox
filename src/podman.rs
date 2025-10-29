use std::error::Error;
use std::{path::PathBuf};

use slurm_spank::{SpankHandle};

use crate::{
    SpankSkyBox,
    is_node_0,
    plugin_err,
};
use sarus_suite_podman_driver::{self as pmd, PodmanCtx};

pub(crate) fn podman_pull_once(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

    if is_node_0(ssb, spank) {
        podman_pull(ssb, spank)?;
    } else {
        podman_pull_wait(ssb, spank)?;
    }

    Ok(())
}

pub(crate) fn podman_pull(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find edf");
        },
    };

    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        },
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

        pmd::parallax_migrate(
            &PathBuf::from(&edf.parallax_path),
            &migrate_ctx,
            &edf.image,
        )?;
        pmd::rmi(&edf.image, Some(&local_ctx));
        
        if pmd::image_exists(&edf.image, Some(&ro_ctx)) {
            podman_pull_done(ssb, spank, -1)?;
            return plugin_err("couldn't find image on imagestore after migrate");
        }
    }

    podman_pull_done(ssb, spank, 0)?;
        
    Ok(())
}


pub(crate) fn podman_pull_done(_ssb: &mut SpankSkyBox, _spank: &mut SpankHandle, _result: i32) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub(crate) fn podman_pull_wait(_ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
    Ok(())
}
