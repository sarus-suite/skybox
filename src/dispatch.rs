use std::error::Error;

use slurm_spank::{Context, Plugin, SpankHandle};

use crate::SpankSkyBox;
use crate::alloc::*;
use crate::slurmd::*;
use crate::slurmstepd::*;
use crate::srun::*;
use crate::{skybox_log_error, skybox_log_user};

const SKYBOX_ERROR_PREFIX: &str = "[skybox] ";

fn format_error_chain(error: &dyn Error) -> String {
    let mut report = error.to_string();
    let mut current = error;

    while let Some(source) = current.source() {
        report.push_str(": ");
        report.push_str(&source.to_string());
        current = source;
    }

    report
}

fn without_skybox_prefix(report: &str) -> &str {
    report.strip_prefix(SKYBOX_ERROR_PREFIX).unwrap_or(report)
}

#[derive(Debug, PartialEq, Eq)]
enum ErrorDestination {
    User,
    Tracing,
}

fn error_destination(context: Context) -> ErrorDestination {
    match context {
        Context::Local | Context::Allocator | Context::Remote => ErrorDestination::User,
        Context::Slurmd | Context::JobScript => ErrorDestination::Tracing,
    }
}

unsafe impl Plugin for SpankSkyBox {
    fn report_error(&self, spank: &mut SpankHandle, error: &dyn Error) {
        let report = format_error_chain(error);

        match spank.context().map(error_destination) {
            Ok(ErrorDestination::User) => {
                skybox_log_user!("{}", without_skybox_prefix(&report))
            }
            Ok(ErrorDestination::Tracing) => tracing::error!("{}", report),
            Err(context_error) => {
                skybox_log_error!(
                    "{}; additionally failed to determine SPANK context: {}",
                    without_skybox_prefix(&report),
                    context_error
                );
            }
        }
    }

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
        if !self.config.skybox_enabled {
            return Ok(());
        }

        match spank.context()? {
            Context::Local => {
                let _ = srun_init_post_opt(self, spank)?;
            }
            Context::Allocator => {
                let _ = alloc_init_post_opt(self, spank)?;
            }
            Context::Remote => {
                let _ = slurmstepd_init_post_opt(self, spank)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn user_init(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        if !self.config.skybox_enabled {
            return Ok(());
        }

        slurmstepd_user_init(self, spank)
    }

    fn task_init(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        if !self.config.skybox_enabled {
            return Ok(());
        }

        slurmstepd_task_init(self, spank)
    }

    fn exit(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        if !self.config.skybox_enabled {
            return Ok(());
        }

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
        if !self.config.skybox_enabled {
            return Ok(());
        }

        slurmd_exit(self, spank)
    }

    fn task_exit(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        if !self.config.skybox_enabled {
            return Ok(());
        }

        slurmstepd_task_exit(self, spank)
    }

    fn task_init_privileged(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        if !self.config.skybox_enabled {
            return Ok(());
        }

        match spank.context()? {
            Context::Remote => {
                let _ = slurmstepd_task_init_privileged(self, spank)?;
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct OuterError {
        source: InnerError,
    }

    impl fmt::Display for OuterError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "outer error")
        }
    }

    impl Error for OuterError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.source)
        }
    }

    #[derive(Debug)]
    struct InnerError;

    impl fmt::Display for InnerError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "inner error")
        }
    }

    impl Error for InnerError {}

    #[test]
    fn formats_error_chain_like_default_reporter() {
        let error = OuterError { source: InnerError };
        assert_eq!(format_error_chain(&error), "outer error: inner error");
    }

    #[test]
    fn removes_one_existing_skybox_prefix() {
        assert_eq!(without_skybox_prefix("[skybox] failure"), "failure");
        assert_eq!(without_skybox_prefix("failure"), "failure");
    }

    #[test]
    fn routes_job_contexts_to_user_log() {
        assert_eq!(error_destination(Context::Local), ErrorDestination::User);
        assert_eq!(
            error_destination(Context::Allocator),
            ErrorDestination::User
        );
        assert_eq!(error_destination(Context::Remote), ErrorDestination::User);
        assert_eq!(
            error_destination(Context::Slurmd),
            ErrorDestination::Tracing
        );
        assert_eq!(
            error_destination(Context::JobScript),
            ErrorDestination::Tracing
        );
    }
}
