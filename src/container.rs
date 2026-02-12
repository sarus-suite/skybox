use std::collections::HashMap;
use std::env::set_current_dir;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
//use std::io::Write;
use cfg_if;
use std::path::Path;

use slurm_spank::{
    SpankError,
    SpankHandle,
    //spank_log_debug,
    //spank_log_error,
    //spank_log_user,
};

use crate::{
    SpankSkyBox,
    //create_folder,
    get_local_task_id,
    //is_local_task_0,
    plugin_err,
    //plugin_string,
    skybox_log_debug,
    skybox_log_error,
};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        pub type PtrT = u8;
    } else {
        pub type PtrT = i8;
    }
}

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
        let userns_path_ptr: *const PtrT = userns_path_c.as_ptr() as *const PtrT;

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
        let mntns_path_ptr: *const PtrT = mntns_path_c.as_ptr() as *const PtrT;

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

    let mut attempts: u32 = 0;
    let pause = std::time::Duration::from_millis(100);
    let max_attempts: u32 = 600;

    loop {
        // Validate the cwd symlink resolves to an actual cwd. If not, return failure string.
        let failure: Option<String> = match std::fs::read_link(&cwd) {
            Ok(target) => {
                if target.is_dir() {
                    None
                } else {
                    Some(format!("cwd link resolved to non-dir target {target:?}"))
                }
            }
            Err(e) => Some(format!("couldn't read cwd link {cwd}: {e}")),
        };

        // Success case
        if failure.is_none() {
            break;
        }

        // Go for retry
        attempts += 1;
        let failure = failure.unwrap();

        // Log first and every 50 retries to limit log spam
        if attempts == 1 || attempts % 50 == 0 {
            skybox_log_debug!("task {} - {failure}, waiting and retrying", get_local_task_id(ssb));
        }

        // Fail with error after max attempts
        if attempts >= max_attempts {
            let msg = format!("failed to open cwd {cwd} after {attempts} attempts: {failure}");
            skybox_log_error!("task {} - {msg}", get_local_task_id(ssb));
            return plugin_err(&msg);
        }

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

    //let msg = plugin_string(format!("changing to {new_workdir}").as_str());
    //spank_log_debug!("{msg}");
    skybox_log_debug!(
        "task {} - changing workdir to {new_workdir}",
        get_local_task_id(ssb)
    );

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
                    //let msg = plugin_string(format!("failed to unset {dvar}: {e}").as_str());
                    //spank_log_error!("{msg}");
                    skybox_log_error!("failed to unset {dvar}: {e}");
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
            //let msg = plugin_string(format!("couldn't open environ {environ_path}: {e}").as_str());
            //spank_log_error!("{msg}");
            skybox_log_error!("couldn't open environ {environ_path}: {e}");
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
                let msg = format!("couldn't parse environ value {string}");
                //spank_log_error!("{msg}");
                skybox_log_error!("{msg}");
                return Err(format!("[{}] {}", crate::get_plugin_name(), msg).into());
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

        let mut overwrite = true;
        if edf_env.contains_key(key) {
            overwrite = false;
        }

        match spank.setenv(key, value, overwrite) {
            Ok(ok) => ok,
            Err(SpankError::EnvExists(_)) => (),
            Err(e) => {
                //let msg = plugin_string(format!("couldn't set env {key}={value}: {e}").as_str());
                //spank_log_error!("{msg}");
                skybox_log_error!("couldn't set env {key}={value}: {e}");
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}
/*
pub(crate) fn container_join_once(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {

    if !is_local_task_0(ssb, spank) {
        return container_join_wait(ssb, spank);
    }

    //Workaround: write pidfile internally
    let pid = ssb.run.clone().unwrap().pid;
    let base_path = ssb.run.clone().unwrap().podman_tmp_path;
    let dir_mode = 0o700;
    let dir_path;
    dir_path = format!("{}", base_path);
    spank_log_debug!("dir_path: {}",&dir_path);
    spank_log_debug!("dir_mode: {}",&dir_mode);
    create_folder(dir_path.clone(), dir_mode)?;
    let internal_pidfile_path = format!("{}/pidfile2", &dir_path);
    spank_log_debug!("internal_pidfile_path: {}",&internal_pidfile_path);
    spank_log_debug!("PID: {}",&pid);
    let mut internal_pidfile = File::create(&internal_pidfile_path)?;
    write!(internal_pidfile, "{}\n", pid)?;

    Ok(())
}

pub(crate) fn container_join_wait(
    ssb: &mut SpankSkyBox,
    _spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    let run = match &ssb.run {
        Some(o) => o,
        None => {
            return plugin_err("couldn't find run");
        }
    };

    let pidfile = format!("{}/pidfile2", run.podman_tmp_path);
    let strpid;

    loop {
        let result = std::fs::read_to_string(&pidfile);
        match result {
            Ok(s) => {
                strpid = s;
                break;
            }
            Err(e) => {
                let msg = plugin_string(
                    format!("couldn't read pidfile yet: {e}, wait 1 sec and retry").as_str(),
                );
                spank_log_debug!("{msg}");

                let pause = std::time::Duration::new(1, 0);
                std::thread::sleep(pause);
            }
        }
    }

    let pid: u64 = strpid.parse()?;

    return Ok(());
}
*/
