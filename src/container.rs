use std::collections::HashMap;
use std::env::set_current_dir;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use slurm_spank::{
    SpankError,
    SpankHandle,
    spank_log_debug,
    spank_log_error,
    //spank_log_user,
};

use crate::{SpankSkyBox, plugin_err, plugin_string};

pub(crate) fn container_join(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let pid = ssb.run.clone().unwrap().pid;

    unsafe {
        // First collect file descriptors for relevant namespaces

        // User namespace
        let userns_path = format!("/proc/{pid}/ns/user");
        let userns_path_c = userns_path.clone() + "\0";
        let userns_path_ptr: *const i8 = userns_path_c.as_ptr() as *const i8;

        let userns_fd = libc::open(userns_path_ptr, libc::O_RDONLY | libc::O_CLOEXEC);
        if userns_fd < 0 {
            //return plugin_err("failed to open userns file");
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
            let msg = format!("failed to open userns file \"{userns_path}\", error: {errno}");
            return plugin_err(&msg);
        }

        // Mount namespace
        let mntns_path = format!("/proc/{pid}/ns/mnt");
        let mntns_path_c = mntns_path.clone() + "\0";
        let mntns_path_ptr: *const i8 = mntns_path_c.as_ptr() as *const i8;

        let mntns_fd = libc::open(mntns_path_ptr, libc::O_RDONLY | libc::O_CLOEXEC);
        if mntns_fd < 0 {
            let msg = format!("failed to open mount namespace file: {mntns_path}");
            return plugin_err(msg.as_str());
        }

        // Then join all relevant namespaces

        // Join user namespace
        let ret = libc::setns(userns_fd, libc::CLONE_NEWUSER);
        if ret < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
            let msg = format!("failed to join user namespace, error: {errno}");
            return plugin_err(&msg);
        }

        // Join mount namespace
        let ret = libc::setns(mntns_fd, libc::CLONE_NEWNS);
        if ret < 0 {
            return plugin_err("failed to join mount namespace");
        }
    }

    Ok(())
}

pub(crate) fn container_wait_cwd(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let pid = ssb.run.clone().unwrap().pid;
    let cwd = format!("/proc/{pid}/cwd");

    while let Err(e) = File::open(&cwd) {
        let msg = plugin_string(format!("couldn't open cwd: {e}, wait 1 sec and retry").as_str());
        spank_log_debug!("{msg}");

        let pause = std::time::Duration::new(1, 0);
        std::thread::sleep(pause);
    }

    Ok(())
}

pub(crate) fn container_set_workdir(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let mut new_workdir = ssb.edf.clone().unwrap().workdir;

    if new_workdir == "" {
        let pid = ssb.run.clone().unwrap().pid;
        new_workdir = format!("/proc/{pid}/cwd");
    }

    let msg = plugin_string(format!("changing to {new_workdir}").as_str());
    spank_log_debug!("{msg}");

    let new_cwd = Path::new(&new_workdir);
    Ok(set_current_dir(&new_cwd)?)
}

pub(crate) fn container_import_env(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let container_deny_env = vec!["LANG", "LANGUAGE", "LC_ALL"];
    let edf_env = ssb.edf.clone().unwrap().env;

    for dvar in container_deny_env {
        if !edf_env.contains_key(dvar) {
            match spank.unsetenv(dvar) {
                Ok(_) => {}
                Err(e) => {
                    let msg = plugin_string(format!("failed to unset {dvar}: {e}").as_str());
                    spank_log_error!("{msg}");
                    return Err(Box::new(e));
                }
            }
        }
    }

    let pid = ssb.run.clone().unwrap().pid;
    let environ_path = format!("/proc/{pid}/environ");
    let environ = Path::new(&environ_path);

    let environ_file = match File::open(&environ) {
        Ok(f) => f,
        Err(e) => {
            let msg = plugin_string(format!("couldn't open environ {environ_path}: {e}").as_str());
            spank_log_error!("{msg}");
            return Err(Box::new(e));
        }
    };

    let mut container_vars = HashMap::new();
    let lines = BufReader::new(environ_file).split(0);
    // Consumes the iterator, returns an (Optional) String
    for line in lines.map_while(Result::ok) {
        let string = String::from_utf8(line)?;
        match string.split_once('=') {
            Some((key, value)) => {
                //spank_log_user!("{} = {}", key, value);
                container_vars.insert(String::from(key), String::from(value));
            }
            None => {
                let msg = plugin_string(format!("couldn't parse environ value {string}").as_str());
                spank_log_error!("{msg}");
                return Err(msg.into());
            }
        }
    }
    //spank_log_user!("{:#?}", container_vars);

    let mut unset_keys = vec![];
    for (key, value) in edf_env.iter() {
        if value == "" {
            unset_keys.push(key);
        }
    }

    for (key, value) in container_vars.iter() {
        if unset_keys.contains(&key) {
            continue;
        }

        let mut overwrite = false;
        if edf_env.contains_key(key) {
            overwrite = true;
        }

        match spank.setenv(key, value, overwrite) {
            Ok(ok) => ok,
            Err(SpankError::EnvExists(_)) => (),
            Err(e) => {
                let msg = plugin_string(format!("couldn't set env {key}={value}: {e}").as_str());
                spank_log_error!("{msg}");
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}
