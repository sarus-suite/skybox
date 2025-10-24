use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;

use slurm_spank::{Plugin, SLURM_VERSION_NUMBER, SPANK_PLUGIN};

use raster::mount::SarusMounts;

pub mod alloc;
pub mod args;
//pub mod config;
pub mod dispatch;
pub mod slurmd;
pub mod slurmstepd;
pub mod srun;

SPANK_PLUGIN!(b"skybox", SLURM_VERSION_NUMBER, SpankSkyBox);

#[derive(Serialize, Default)]
struct SpankSkyBox {
    container_image: Option<String>,
    container_mounts: Option<SarusMounts>,
    container_workdir: Option<String>,
    container_name: Option<String>,
    container_name_flags: Option<String>,
    container_save: Option<String>,
    container_mount_home: Option<bool>,
    container_remap_root: Option<bool>,
    container_entrypoint: Option<bool>,
    container_entrypoint_log: Option<bool>,
    container_writable: Option<bool>,
    container_env: Option<HashMap<String, String>>,
    environment: Option<String>,
    dump_environment: Option<bool>,
}

pub(crate) fn get_plugin_name() -> String {
    return String::from("skybox");
}

pub(crate) fn plugin_string(s: &str) -> String {
    return format!("[{}] {}", get_plugin_name(), s);
}

pub(crate) fn plugin_err(s: &str) -> Result<(), Box<dyn Error>> {
    return Err(plugin_string(s).into());
}
