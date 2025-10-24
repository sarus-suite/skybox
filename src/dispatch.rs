use std::error::Error;

use slurm_spank::{Context, Plugin, SpankHandle};

use crate::SpankSkyBox;
use crate::alloc::*;
use crate::slurmd::*;
use crate::slurmstepd::*;
use crate::srun::*;

unsafe impl Plugin for SpankSkyBox {
    fn init(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Slurmd => {
                let _ = slurmd_init(self, spank)?;
            }
            Context::Local => {
                let _ = srun_init(self, spank)?;
            }
            Context::Allocator => {
                let _ = alloc_init(self, spank)?;
            }
            Context::Remote => {
                let _ = slurmstepd_init(self, spank)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn init_post_opt(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Local => {
                let _ = srun_post_opt(self, spank)?;
            }
            Context::Allocator => {
                let _ = alloc_post_opt(self, spank)?;
            }
            Context::Remote => {
                let _ = slurmstepd_post_opt(self, spank)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn exit(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        match spank.context()? {
            Context::Slurmd => {
                let _ = slurmd_exit(self, spank)?;
            }
            Context::Local => {
                let _ = srun_exit(self, spank)?;
            }
            Context::Allocator => {
                let _ = alloc_exit(self, spank)?;
            }
            Context::Remote => {
                let _ = slurmstepd_exit(self, spank)?;
            }
            _ => {}
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
            }
            _ => {}
        }

        Ok(())
    }
}
