use std::error::Error;
use std::{path::PathBuf};
use std::fs::File;
use std::io::Write;
use std::io::{BufReader, BufRead};

use slurm_spank::{
    SpankHandle,
    spank_log_error,
    spank_log_info,
};

use crate::{
    SpankSkyBox,
    is_node_0,
    is_task_0,
    plugin_err,
    plugin_string,
};
use sarus_suite_podman_driver::{self as pmd, PodmanCtx};

pub(crate) fn podman_pull_once(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

    if is_task_0(ssb, spank) && is_node_0(ssb, spank) {
        match podman_pull(ssb, spank) {
            Ok(_) => {
                podman_pull_done(ssb, spank, 0)?;
            },
            Err(e) => {
                spank_log_error!("{}", plugin_string(format!("{}",e).as_str()));
                podman_pull_done(ssb, spank, -1)?;
            },
        }
    } else {
        podman_pull_wait(ssb, spank)?;
    }

    Ok(())
}

pub(crate) fn podman_pull(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

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
        
        if !pmd::image_exists(&edf.image, Some(&ro_ctx)) {
            return plugin_err("couldn't find image on imagestore after migrate");
        }
    }

        
    Ok(())
}


pub(crate) fn podman_pull_done(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle, result: i32) -> Result<(), Box<dyn Error>> {
   
    let msg = plugin_string(format!("image importer completed with {} - communicating", result).as_str());
   spank_log_info!("{msg}");

    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        },
    };

    let mut file = File::create(run.syncfile_path)?;
    write!(file, "{}\n", result)?;

    if result != 0 {
        return plugin_err("");
    } else {
        return Ok(());
    }
}

pub(crate) fn podman_pull_wait(ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

    let msg1 = plugin_string("waiting on image importer");
    spank_log_info!("{msg1}");
    
    let run = match ssb.run.clone() {
        Some(r) => r,
        None => {
            return plugin_err("cannot find run structure");
        },
    };

    let pause = std::time::Duration::new(1,0);
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
