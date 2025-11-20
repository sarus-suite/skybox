use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use slurm_spank::{SpankHandle, spank_log_user};

use crate::SpankSkyBox;
use crate::args::*;
use crate::edf::*;

#[allow(unused_variables)]
pub(crate) fn alloc_init(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    register_plugin_args(spank)?;
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_init_post_opt(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    load_plugin_args(plugin, spank)?;
    load_edf(plugin, spank)?;
    set_remaining_default_args(plugin)?;

    //skybox_log_context(plugin);
    sbatch_warn_msg(plugin, spank);
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_exit(
    plugin: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub(crate) fn sbatch_warn_msg(plugin: &mut SpankSkyBox, _spank: &mut SpankHandle) -> () {
    if plugin.args.edf.is_none() {
        return ();
    };

    let prog = env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
        .unwrap();

    let unpadded = format!("\"{}\" is still considered experimental", prog);
    let warnmsg = format!(
        "
--------------------------------------------------------------------------------
| Use of the \"--edf\" option for {:<width$}|
| and could result in unexpected behavior.                                     |
| Use of \"--edf\" is currently only recommended for the \"srun\" command.         |
|                                                                              |
| Please read carefully the Container Engine page on the CSCS Knowledge Base.  |
--------------------------------------------------------------------------------
",
        unpadded,
        width = 47
    );

    spank_log_user!("{warnmsg}");
}
