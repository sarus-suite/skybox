use std::error::Error;

use slurm_spank::{
    spank_log_verbose,
    spank_log_user,
    Context,
    Plugin,
    SpankHandle,
};

use crate::SpankSkyBox;
use crate::args::*;

unsafe impl Plugin for SpankSkyBox {
    fn init(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {

        match spank.context()? {
            Context::Slurmd => {
                let _ = slurmd_init(self, spank)?;
            },
            Context::Local => {
                let _ = srun_init(self, spank)?;
            },
            Context::Allocator => {
                let _ = alloc_init(self, spank)?;
            },
            Context::Remote => {
                let _ = slurmstepd_init(self, spank)?;
            },
            _ => {},
        }

        Ok(())
    }
    
    fn init_post_opt(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Local => {
                let _ = srun_post_opt(self, spank)?;
            },
            Context::Allocator => {
                let _ = alloc_post_opt(self, spank)?;
            },
            Context::Remote => {
                let _ = slurmstepd_post_opt(self, spank)?;
            },
            _ => {},
        }

        Ok(())
    }

    fn exit(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Slurmd => {
                let _ = slurmd_exit(self, spank)?;
            },
            Context::Local => {
                let _ = srun_exit(self, spank)?;
            },
            Context::Allocator => {
                let _ = alloc_exit(self, spank)?;
            },
            Context::Remote => {
                let _ = slurmstepd_exit(self, spank)?;
            },
            _ => {},
        }

        Ok(())
    }

    fn slurmd_exit(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        slurmd_exit(self, spank)
    }    
    
    fn task_init_privileged(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Remote => {
                let _ = slurmstepd_task_init_privileged(self, spank)?;
            },
            _ => {}
        }

        Ok(())
    }
}

#[allow(unused_variables)]
pub(crate) fn slurmd_init(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn srun_init(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    let r = register_plugin_args(spank)?;
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn alloc_init(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_init(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    let r = register_plugin_args(spank)?;
    Ok(r)
}

#[allow(unused_variables)]
pub(crate) fn srun_post_opt(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    let _ = load_plugin_args(plugin, spank)?;

    spank_log_user!("computed args:");
    spank_log_user!(
        "{}",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_post_opt(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_post_opt(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    let _ = load_plugin_args(plugin, spank)?;

    spank_log_verbose!("{}: computed args:", "skybox");
    spank_log_verbose!(
        "{}: {}",
        "skybox",
        serde_json::to_string_pretty(&plugin).unwrap_or(String::from("ERROR"))
    );

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmd_exit(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn srun_exit(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn alloc_exit(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_exit(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn slurmstepd_task_init_privileged(plugin: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>>  {
    Ok(())
}

