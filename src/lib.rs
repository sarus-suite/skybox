use std::error::Error;
use serde::{Serialize};

use slurm_spank::{
    spank_log_verbose,
    spank_log_user,
    Context,
    Plugin,
    SpankHandle,
    SpankOption,
    SLURM_VERSION_NUMBER,
    SPANK_PLUGIN,
};

// All spank plugins must define this macro for the
// Slurm plugin loader.
SPANK_PLUGIN!(b"skybox", SLURM_VERSION_NUMBER, SpankSkyBox);

#[derive(Serialize, Default)]
struct SpankSkyBox {
    container_image: Option<String>,
    container_mounts: Option<String>,
    container_workdir: Option<String>,
    container_name: Option<String>,
    container_save: Option<String>,
    container_mount_home: Option<bool>,
    container_remap_root: Option<bool>,
    container_entrypoint: Option<bool>,
    container_entrypoint_log: Option<bool>,
    container_writable: Option<bool>,
    container_env: Option<String>,
    environment: Option<String>,
    dump_environment: Option<bool>,
}

struct SpankArg {
    name: String,
    value: String,
    usage: String,
    has_arg: bool,
}

type SpankArgs = Vec<SpankArg>;

fn add_arg(mut v: SpankArgs, a: SpankArg) -> SpankArgs {
    v.push(a);
    v
}

fn register_plugin_args(spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
    let plug_name = "skybox";

    let mut opts = vec!();
    opts = add_arg(opts, SpankArg {
        name: String::from("container-image"),
        value: String::from("[USER@][REGISTRY#]IMAGE[:TAG]|PATH"),
        usage: String::from("the image to use for the container filesystem. Can be either a docker image given as an enroot URI, or a path to a squashfs file on the remote host filesystem."),
        has_arg: true,
    });
    
    opts = add_arg(opts, SpankArg {
        name: String::from("container-mounts"),
        value: String::from("SRC:DST[:FLAGS][,SRC:DST...]"),
        usage: String::from("bind mount[s] inside the container. Mount flags are separated with \"+\", e.g. \"ro+rprivate\""),
        has_arg: true,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-workdir"),
        value: String::from("PATH"),
        usage: String::from("working directory inside the container"),
        has_arg: true,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-name"),
        value: String::from("NAME"),
        usage: String::from("name to use for saving and loading the container on the host. Unnamed containers are removed after the slurm task is complete; named containers are not. If a container with this name already exists, the existing container is used and the import is skipped."),
        has_arg: true,
    });
    
    opts = add_arg(opts, SpankArg {
        name: String::from("container-save"),
        value: String::from("PATH"),
        usage: String::from("Save the container state to a squashfs file on the remote host filesystem."),
        has_arg: true,
    });
    
    opts = add_arg(opts, SpankArg {
        name: String::from("container-mount-home"),
        value: String::from(""),
        usage: String::from("bind mount the user's home directory. System-level enroot settings might cause this directory to be already-mounted."),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("no-container-mount-home"),
        value: String::from(""),
        usage: String::from("do not bind mount the user's home directory"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-remap-root"),
        value: String::from(""),
        usage: String::from("ask to be remapped to root inside the container. Does not grant elevated system permissions, despite appearances."),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("no-container-remap-root"),
        value: String::from(""),
        usage: String::from("do not remap to root inside the container"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-entrypoint"),
        value: String::from(""),
        usage: String::from("execute the entrypoint from the container image"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("no-container-entrypoint"),
        value: String::from(""),
        usage: String::from("do not execute the entrypoint from the container image"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-entrypoint-log"),
        value: String::from(""),
        usage: String::from("print the output of the entrypoint script"),
        has_arg: false,
    });
    
    opts = add_arg(opts, SpankArg {
        name: String::from("container-writable"),
        value: String::from(""),
        usage: String::from("make the container filesystem writable"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-readonly"),
        value: String::from(""),
        usage: String::from("make the container filesystem read-only"),
        has_arg: false,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("container-env"),
        value: String::from("NAME[,NAME...]"),
        usage: String::from("names of environment variables to override with the host environment and set at the entrypoint. By default, all exported host environment variables are set in the container after the entrypoint is run, but their existing values in the image take precedence; the variables specified with this flag are preserved from the host and set before the entrypoint runs"),
        has_arg: true,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("environment"),
        value: String::from("PATH"),
        usage: String::from("the path to the Environment Definition File to use."),
        has_arg: true,
    });

    opts = add_arg(opts, SpankArg {
        name: String::from("dump-environment"),
        value: String::from(""),
        usage: String::from("dumps the final values of the environment keeping into account base_environment and command line overrides."),
        has_arg: false,
    });

    for opt in opts {
        let so;
        if opt.has_arg {
            so = SpankOption::new(&opt.name)
                        .takes_value(&opt.value)
                        .usage(format!("[{}] {}", plug_name, &opt.usage).as_str());
        } else {
            so = SpankOption::new(&opt.name)
                        .usage(format!("[{}] {}", plug_name, &opt.usage).as_str());
        }
        spank.register_option(so)?;
    }
    Ok(())
}

fn set_arg_mount_home(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_mount_home {
        Some(_) => return Err("container-mount-home argument specified more than once".into()),
        None => {
            ssb.container_mount_home = Some(value);
        },
    }
    Ok(())
}

fn set_arg_remap_root(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_remap_root {
        Some(_) => return Err("container-remap-root argument specified more than once".into()),
        None => {
            ssb.container_remap_root = Some(value);
        },
    }
    Ok(())
}

fn set_arg_entrypoint(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_entrypoint {
        Some(_) => return Err("container-entrypoint argument specified more than once".into()),
        None => {
            ssb.container_entrypoint = Some(value);
        },
    }
    Ok(())
}

fn set_arg_entrypoint_log(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_entrypoint_log {
        Some(_) => return Err("container-entrypoint-log argument specified more than once".into()),
        None => {
            ssb.container_entrypoint_log = Some(value);
        },
    }
    Ok(())
}

fn set_arg_writable(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_writable {
        Some(_) => return Err("container-writable argument specified more than once".into()),
        None => {
            ssb.container_writable = Some(value);
        },
    }
    Ok(())
}

fn set_arg_dump_environment(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.dump_environment {
        Some(_) => return Err("dump-environment argument specified more than once".into()),
        None => {
            ssb.dump_environment = Some(value);
        },
    }
    Ok(())
}

fn load_plugin_args(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
    if spank.is_option_set("container-image") {
        ssb.container_image = spank
            .get_option_value("container-image")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("container-mounts") {
        ssb.container_mounts = spank
            .get_option_value("container-mounts")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("container-workdir") {
        ssb.container_workdir = spank
            .get_option_value("container-workdir")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("container-name") {
        ssb.container_name = spank
            .get_option_value("container-name")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("container-save") {
        ssb.container_save = spank
            .get_option_value("container-save")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("container-mount-home") {
        let _ = set_arg_mount_home(ssb, true);
    }

    if spank.is_option_set("no-container-mount-home") {
        let _ = set_arg_mount_home(ssb, false);
    }

    if spank.is_option_set("container-remap-root") {
        let _ = set_arg_remap_root(ssb, true);
    }

    if spank.is_option_set("no-container-remap-root") {
        let _ = set_arg_remap_root(ssb, false);
    }

    if spank.is_option_set("container-entrypoint") {
        let _ = set_arg_entrypoint(ssb, true);
    }

    if spank.is_option_set("no-container-entrypoint") {
        let _ = set_arg_entrypoint(ssb, false);
    }

    if spank.is_option_set("container-entrypoint-log") {
        let _ = set_arg_entrypoint_log(ssb, true);
    }
    
    if spank.is_option_set("container-writable") {
        let _ = set_arg_writable(ssb, true);
    }

    if spank.is_option_set("container-readonly") {
        let _ = set_arg_writable(ssb, false);
    }

    if spank.is_option_set("container-env") {
        ssb.container_env = spank
            .get_option_value("container-env")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("environment") {
        ssb.environment = spank
            .get_option_value("environment")?
            .map(|s| s.to_string());
    }

    if spank.is_option_set("dump-environment") {
        let _ = set_arg_dump_environment(ssb, true);
    }

    Ok(())
}

unsafe impl Plugin for SpankSkyBox {
    fn init(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        
        match spank.context()? {
            Context::Local | Context::Remote => {
                let _ = register_plugin_args(spank)?;
            }
            _ => {}
        }

        Ok(())
    }
    fn init_post_opt(&mut self, spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        let _ = load_plugin_args(self, spank)?;

        spank_log_verbose!("{}: computed args:", "skybox");
        spank_log_verbose!(
            "{}: {}",
            "skybox",
            serde_json::to_string_pretty(&self).unwrap_or(String::from("ERROR"))
        );

        Ok(())
    }

    fn user_init(&mut self, _spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
        spank_log_user!("computed args:");
        spank_log_user!(
            "{}",
            serde_json::to_string_pretty(&self).unwrap_or(String::from("ERROR"))
        );

        Ok(())
    }

}
