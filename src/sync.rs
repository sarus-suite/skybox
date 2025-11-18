use std::error::Error;
use std::path::Path;

use slurm_spank::{
    SpankHandle,
};

use crate::{
    SpankSkyBox,
    plugin_err,
};

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
                let msg = format!("couldn't cleanup \"{:#?}\", error {}", &completed_dir_path, e);
                return plugin_err(&msg);
            }
        };
    }
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
    Ok(())
}
