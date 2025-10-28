use serde::{Serialize, Deserialize};
use slurm_spank::{SpankHandle, SpankOption};
//use std::collections::HashMap;
use std::error::Error;

use crate::{SpankSkyBox, get_plugin_name, plugin_err};
//use raster::mount::{SarusMounts, sarus_mounts_from_strings};

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct SkyBoxArgs {
    pub(crate) edf: Option<String>,
}

pub(crate) struct SpankArg {
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

pub(crate) fn register_plugin_args(spank: &mut SpankHandle) -> Result<(), Box<dyn Error>> {
    let plug_name = get_plugin_name();

    let mut opts = vec![];
    /*
    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-image"),
            value: String::from("[USER@][REGISTRY#]IMAGE[:TAG]|PATH"),
            usage: String::from(
                "the image to use for the container filesystem. Can be either a docker image given as an enroot URI, or a path to a squashfs file on the remote host filesystem.",
            ),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-mounts"),
            value: String::from("SRC:DST[:FLAGS][,SRC:DST...]"),
            usage: String::from(
                "bind mount[s] inside the container. Mount flags are separated with \"+\", e.g. \"ro+rprivate\"",
            ),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-workdir"),
            value: String::from("PATH"),
            usage: String::from("working directory inside the container"),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-name"),
            value: String::from("NAME"),
            usage: String::from(
                "name to use for saving and loading the container on the host. Unnamed containers are removed after the slurm task is complete; named containers are not. If a container with this name already exists, the existing container is used and the import is skipped.",
            ),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-save"),
            value: String::from("PATH"),
            usage: String::from(
                "Save the container state to a squashfs file on the remote host filesystem.",
            ),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-mount-home"),
            value: String::from(""),
            usage: String::from(
                "bind mount the user's home directory. System-level enroot settings might cause this directory to be already-mounted.",
            ),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("no-container-mount-home"),
            value: String::from(""),
            usage: String::from("do not bind mount the user's home directory"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-remap-root"),
            value: String::from(""),
            usage: String::from(
                "ask to be remapped to root inside the container. Does not grant elevated system permissions, despite appearances.",
            ),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("no-container-remap-root"),
            value: String::from(""),
            usage: String::from("do not remap to root inside the container"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-entrypoint"),
            value: String::from(""),
            usage: String::from("execute the entrypoint from the container image"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("no-container-entrypoint"),
            value: String::from(""),
            usage: String::from("do not execute the entrypoint from the container image"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-entrypoint-log"),
            value: String::from(""),
            usage: String::from("print the output of the entrypoint script"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-writable"),
            value: String::from(""),
            usage: String::from("make the container filesystem writable"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-readonly"),
            value: String::from(""),
            usage: String::from("make the container filesystem read-only"),
            has_arg: false,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("container-env"),
            value: String::from("NAME[,NAME...]"),
            usage: String::from(
                "names of environment variables to override with the host environment and set at the entrypoint. By default, all exported host environment variables are set in the container after the entrypoint is run, but their existing values in the image take precedence; the variables specified with this flag are preserved from the host and set before the entrypoint runs",
            ),
            has_arg: true,
        },
    );

    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("environment"),
            value: String::from("PATH"),
            usage: String::from("the path to the Environment Definition File to use."),
            has_arg: true,
        },
    );
    */
    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("edf"),
            value: String::from("PATH"),
            usage: String::from("the path to the Environment Definition File to use."),
            has_arg: true,
        },
    );
    /*    
    opts = add_arg(
        opts,
        SpankArg {
            name: String::from("dump-environment"),
            value: String::from(""),
            usage: String::from(
                "dumps the final values of the environment keeping into account base_environment and command line overrides.",
            ),
            has_arg: false,
        },
    );
    */

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
/*
pub(crate) fn set_arg_mount_home(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_mount_home {
        Some(_) => {
            if ssb.container_mount_home != Some(value) {
                plugin_err(
                    "both --container-mount-home and --no-container-mount-home were specified",
                )?
            }
        }
        None => {
            ssb.container_mount_home = Some(value);
        }
    }
    Ok(())
}

pub(crate) fn set_arg_remap_root(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_remap_root {
        Some(_) => {
            if ssb.container_remap_root != Some(value) {
                plugin_err(
                    "both --container-remap-root and --no-container-remap-root were specified",
                )?
            }
        }
        None => {
            ssb.container_remap_root = Some(value);
        }
    }
    Ok(())
}

pub(crate) fn set_arg_entrypoint(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_entrypoint {
        Some(_) => {
            if ssb.container_entrypoint != Some(value) {
                plugin_err(
                    "both --container-entrypoint and --no-container-entrypoint were specified",
                )?
            }
        }
        None => {
            ssb.container_entrypoint = Some(value);
        }
    }
    Ok(())
}

pub(crate) fn set_arg_entrypoint_log(
    ssb: &mut SpankSkyBox,
    value: bool,
) -> Result<(), Box<dyn Error>> {
    ssb.container_entrypoint_log = Some(value);
    Ok(())
}

pub(crate) fn set_arg_env_vars(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    let vars = get_env_vars_from_string(value)?;
    ssb.container_env = Some(vars);
    Ok(())
}

pub(crate) fn set_arg_environment(
    ssb: &mut SpankSkyBox,
    value: String,
) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--environment: argument required")?
    }
    ssb.environment = Some(value);
    Ok(())
}
*/

pub(crate) fn set_arg_edf(
    ssb: &mut SpankSkyBox,
    value: String,
) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--edf: argument required")?
    }
    //ssb.environment = Some(value.clone());
    ssb.args.edf = Some(value);
    Ok(())
}
/*
pub(crate) fn set_arg_image(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--container-image: argument required")?
    }
    /* // No WAY it can happen.
    match ssb.container_image {
        Some(_) => if ssb.container_image != Some(value) {
            plugin_err("--container-image specified multiple times")?
        },
        None => {
            ssb.container_image = Some(value);
        },
    }
    */
    ssb.container_image = Some(value);
    Ok(())
}

pub(crate) fn set_arg_mounts(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    let mounts = get_mounts_from_string(value)?;
    ssb.container_mounts = Some(mounts);
    Ok(())
}

pub(crate) fn set_arg_name(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--container-name: argument required")?
    }
    (ssb.container_name, ssb.container_name_flags) = get_name_and_flags(value)?;
    Ok(())
}

pub(crate) fn set_arg_save(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--container-save: argument required")?
    }
    if value.ends_with('/') {
        plugin_err("--container-save: target is a directory")?
    }
    ssb.container_save = Some(value);
    Ok(())
}

pub(crate) fn set_arg_workdir(ssb: &mut SpankSkyBox, value: String) -> Result<(), Box<dyn Error>> {
    if value == "" {
        plugin_err("--container-workdir: argument required")?
    }
    ssb.container_workdir = Some(value);
    Ok(())
}

pub(crate) fn set_arg_writable(ssb: &mut SpankSkyBox, value: bool) -> Result<(), Box<dyn Error>> {
    match ssb.container_writable {
        Some(_) => {
            if ssb.container_writable != Some(value) {
                plugin_err("both --container-writable and --container-readonly were specified")?
            }
        }
        None => {
            ssb.container_writable = Some(value);
        }
    }
    Ok(())
}

pub(crate) fn set_arg_dump_environment(
    ssb: &mut SpankSkyBox,
    value: bool,
) -> Result<(), Box<dyn Error>> {
    match ssb.dump_environment {
        Some(_) => return Err("dump-environment argument specified more than once".into()),
        None => {
            ssb.dump_environment = Some(value);
        }
    }
    Ok(())
}
*/
pub(crate) fn load_plugin_args(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    /*
    WARNING:
    There is a difference from pyxis behaviour, using spank_slurm rust
    library. By using is_/get_option methods there is no way to parse
    the same argument multiple times. In these cases pyxis fails,
    but here we consider the last entry as a good one.
    */
    /*
    if spank.is_option_set("container-image") {
        let arg_value = spank
            .get_option_value("container-image")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_image(ssb, arg_value)?;
    }

    if spank.is_option_set("container-mounts") {
        let arg_value = spank
            .get_option_value("container-mounts")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_mounts(ssb, arg_value)?;
    }

    if spank.is_option_set("container-workdir") {
        let arg_value = spank
            .get_option_value("container-workdir")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_workdir(ssb, arg_value)?;
    }

    if spank.is_option_set("container-name") {
        let arg_value = spank
            .get_option_value("container-name")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_name(ssb, arg_value)?;
    }

    if spank.is_option_set("container-save") {
        let arg_value = spank
            .get_option_value("container-save")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_save(ssb, arg_value)?;
    }

    if spank.is_option_set("container-mount-home") {
        let _ = set_arg_mount_home(ssb, true)?;
    }

    if spank.is_option_set("no-container-mount-home") {
        let _ = set_arg_mount_home(ssb, false)?;
    }

    if spank.is_option_set("container-remap-root") {
        let _ = set_arg_remap_root(ssb, true)?;
    }

    if spank.is_option_set("no-container-remap-root") {
        let _ = set_arg_remap_root(ssb, false)?;
    }

    if spank.is_option_set("container-entrypoint") {
        let _ = set_arg_entrypoint(ssb, true)?;
    }

    if spank.is_option_set("no-container-entrypoint") {
        let _ = set_arg_entrypoint(ssb, false)?;
    }

    if spank.is_option_set("container-entrypoint-log") {
        let _ = set_arg_entrypoint_log(ssb, true)?;
    }

    if spank.is_option_set("container-writable") {
        let _ = set_arg_writable(ssb, true)?;
    }

    if spank.is_option_set("container-readonly") {
        let _ = set_arg_writable(ssb, false)?;
        /*
        match set_arg_writable(ssb, false) {
            Ok(_) => {},
            Err(e) => {
               //spank_log_error!("{e}");
               return Err(e);
            },
        }
        */
    }

    if spank.is_option_set("container-env") {
        let arg_value = spank
            .get_option_value("container-env")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_env_vars(ssb, arg_value)?;
    }

    if spank.is_option_set("environment") {
        let arg_value = spank
            .get_option_value("environment")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_environment(ssb, arg_value)?;
    }
    */

    if spank.is_option_set("edf") {
        let arg_value = spank
            .get_option_value("edf")?
            .map(|s| s.to_string())
            .unwrap();
        let _ = set_arg_edf(ssb, arg_value)?;
    }
    
    /*
    if spank.is_option_set("dump-environment") {
        let _ = set_arg_dump_environment(ssb, true)?;
    }
    */

    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn set_remaining_default_args(ssb: &mut SpankSkyBox) -> Result<(), Box<dyn Error>> {
    /*
    match ssb.container_mount_home {
        None => ssb.container_mount_home = Some(false),
        Some(_) => {}
    }
    match ssb.container_remap_root {
        None => ssb.container_remap_root = Some(false),
        Some(_) => {}
    }
    match ssb.container_entrypoint {
        None => ssb.container_entrypoint = Some(false),
        Some(_) => {}
    }
    match ssb.container_entrypoint_log {
        None => ssb.container_entrypoint_log = Some(false),
        Some(_) => {}
    }
    match ssb.container_writable {
        None => ssb.container_writable = Some(false),
        Some(_) => {}
    }
    match ssb.dump_environment {
        None => ssb.dump_environment = Some(false),
        Some(_) => {}
    }
    */

    Ok(())
}
/*
pub(crate) fn get_mounts_from_string(input: String) -> Result<SarusMounts, Box<dyn Error>> {
    let v = input.split(',').map(|x| x.to_string()).collect();
    let mounts = sarus_mounts_from_strings(v)?;
    Ok(mounts)
}
pub(crate) fn get_env_vars_from_string(
    input: String,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut h = HashMap::from([]);
    let v: Vec<String> = input.split(',').map(|x| x.to_string()).collect();
    for i in v.iter() {
        let mut split = i.split('=');
        if split.clone().count() != 2 {
            plugin_err(format!("--container-env: invalid format: {}", input).as_ref())?
        }
        let key = split.next().unwrap();
        if key == "" {
            plugin_err(format!("--container-env: invalid format: {}", input).as_ref())?
        }
        let value = split.next().unwrap();
        h.insert(key.to_string(), value.to_string());
    }
    Ok(h)
}

pub(crate) fn get_name_and_flags(
    input: String,
) -> Result<(Option<String>, Option<String>), Box<dyn Error>> {
    let allowed_flags = vec!["auto", "create", "exec", "no_exec"];
    let split: Vec<String> = input.split(':').map(|x| x.to_string()).collect();
    let count = split.len();

    let mut name = None;
    let mut flags = None;

    if count > 2 {
        plugin_err(format!("--container-name: invalid format: {}", input).as_ref())?
    } else if count == 2 {
        name = Some(split[0].clone());
        flags = Some(split[1].clone());
    } else if count == 1 {
        name = Some(split[0].clone());
    }

    if name.is_none() || name == Some("".to_string()) {
        plugin_err("--container-name: empty name")?
    }

    if flags.is_none() || flags == Some("".to_string()) {
        flags = Some(String::from("auto"));
    } else {
        let flags_split: Vec<String> = flags
            .clone()
            .unwrap()
            .split(',')
            .map(|x| x.to_string())
            .collect();
        for f in flags_split {
            if !allowed_flags.contains(&(f.as_str())) {
                plugin_err(
                    "--container-name: flag must be \"auto\", \"create\", \"exec\" or \"no_exec\"",
                )?
            }
        }
    }

    Ok((name, flags))
}
*/
