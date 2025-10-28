//use serde::{Serialize, Deserialize};
use slurm_spank::{
    Context,
    SpankHandle,
   //spank_log_user
};
//use std::collections::HashMap;
use std::error::Error;

use crate::{SpankSkyBox, spank_getenv};
//use raster::{EDF};

/*
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct SkyBoxEDF {
    pub(crate) edf: Option<EDF>,
}
*/

pub(crate) fn load_environment(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {

    let edf_name = match &ssb.args.edf {
        Some(name) => String::from(name),
        None => {
            return Ok(());
        },
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

fn spank_remote_edf_render(path: String, spank: &mut SpankHandle) -> Result<raster::EDF, Box<dyn Error>> {
    let sp = spank_remote_get_search_paths(spank);
    Ok(raster::render_from_search_paths(path, sp)?)
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
