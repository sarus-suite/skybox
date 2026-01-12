use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::time::Duration;
use users::get_current_groupname;

use slurm_spank::{Context, SpankHandle};

use crate::sync::sync_load_dynconf;
use crate::{SpankSkyBox, get_job_env, skybox_log_debug};

#[derive(Deserialize, Debug)]
struct DynConfRaw {
    plugin: Option<String>,
    engine: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(crate) struct DynConf {
    plugin: String,
    engine: String,
}

pub(crate) fn load_dynconf(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) {
    let dynconf_url = ssb.config.dynconf_url.clone();
    let dynconf_path = ssb.config.dynconf_path.clone();

    if dynconf_url == "" || dynconf_path == "" {
        return;
    }

    let spank_context = match spank.context() {
        Ok(ok) => ok,
        Err(_) => {
            return;
        }
    };

    match spank_context {
        Context::Local => {
            load_dynconf_url(ssb, spank);
        }
        Context::Allocator => {
            load_dynconf_url(ssb, spank);
        }
        Context::Remote => {
            load_dynconf_slurmstepd(ssb, spank);
        }
        _ => {
            return;
        }
    }
}

pub(crate) fn load_dynconf_url(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> DynConf {
    let empty_dynconf = DynConf {
        plugin: String::from(""),
        engine: String::from(""),
    };

    let spank_context = match spank.context() {
        Ok(ok) => ok,
        Err(e) => {
            skybox_log_debug!("cannot retrieve context: {}", e);
            return empty_dynconf;
        }
    };

    //let mut dynconf_input = HashMap::new();
    let dynconf_input;

    match spank_context {
        Context::Local => {
            dynconf_input = build_local_input(ssb, spank);
        }
        Context::Allocator | Context::Remote => {
            dynconf_input = build_job_input(ssb, spank);
        }
        _ => {
            return empty_dynconf;
        }
    }

    let dynconf_url = ssb.config.dynconf_url.clone();

    skybox_log_debug!("Querying dynconf at {}", dynconf_url);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    let dynconf_output = client.post(dynconf_url).json(&dynconf_input).send();

    let output = match dynconf_output {
        Ok(ok) => ok,
        Err(e) => {
            skybox_log_debug!("dynconf response ERROR: {}", e);
            return empty_dynconf;
        }
    };

    let dynconfraw: DynConfRaw = match output.json() {
        Ok(ok) => ok,
        Err(e) => {
            skybox_log_debug!("dynconf response ERROR: {}", e);
            return empty_dynconf;
        }
    };

    let plugin = match dynconfraw.plugin {
        Some(s) => s,
        None => String::from(""),
    };

    let engine = match dynconfraw.engine {
        Some(s) => s,
        None => String::from(""),
    };

    let dynconf = DynConf {
        plugin: plugin,
        engine: engine,
    };

    skybox_log_debug!("dynconf response: {:#?}", dynconf);

    apply_dynconf(ssb, dynconf.clone());

    return dynconf;
}

fn load_dynconf_slurmstepd(ssb: &mut SpankSkyBox, spank: &mut SpankHandle) {
    sync_load_dynconf(ssb, spank);
}

pub(crate) fn apply_dynconf(ssb: &mut SpankSkyBox, dynconf: DynConf) {
    if dynconf.plugin == "skybox" {
        ssb.config.skybox_enabled = true;
    } else if dynconf.plugin != "" {
        ssb.config.skybox_enabled = false;
    }
}

fn build_job_input(_ssb: &mut SpankSkyBox, spank: &mut SpankHandle) -> HashMap<String, String> {
    let mut dynconf_input = HashMap::new();

    let env = get_job_env(spank);

    let unknown_job = String::from("0");
    let unknown = String::from("UNKNOWN");
    let job = env.get("SLURM_JOB_ID").unwrap_or(&unknown_job);
    let system = env.get("SLURM_CLUSTER_NAME").unwrap_or(&unknown);
    let account = env.get("SLURM_JOB_ACCOUNT").unwrap_or(&unknown);
    let user = env.get("SLURM_JOB_USER").unwrap_or(&unknown);

    dynconf_input.insert("job".to_string(), job.to_string());
    dynconf_input.insert("system".to_string(), system.to_string());
    dynconf_input.insert("account".to_string(), account.to_string());
    dynconf_input.insert("user".to_string(), user.to_string());

    /*
    skybox_log_debug!("JOB: {}", dynconf_input.get("job").unwrap());
    skybox_log_debug!("SYSTEM: {}", dynconf_input.get("system").unwrap());
    skybox_log_debug!("ACCOUNT: {}", dynconf_input.get("account").unwrap());
    skybox_log_debug!("USER: {}", dynconf_input.get("user").unwrap());
    */

    return dynconf_input;
}

fn build_local_input(_ssb: &mut SpankSkyBox, _spank: &mut SpankHandle) -> HashMap<String, String> {
    let mut dynconf_input = HashMap::new();

    let unknown_job = String::from("0");
    let unknown = String::from("UNKNOWN");
    let job = env::var("SLURM_JOB_ID").unwrap_or(unknown_job);
    let system = env::var("CLUSTER_NAME").unwrap_or(unknown.clone());
    let account: String = get_current_groupname()
        .unwrap_or(OsString::from("UNKNOWN"))
        .into_string()
        .unwrap_or(unknown.clone());
    let user = env::var("USER").unwrap_or(unknown.clone());

    dynconf_input.insert("job".to_string(), job.to_string());
    dynconf_input.insert("system".to_string(), system.to_string());
    dynconf_input.insert("account".to_string(), account.to_string());
    dynconf_input.insert("user".to_string(), user.to_string());

    /*
    spank_log_user!("JOB: {}", dynconf_input.get("job").unwrap());
    spank_log_user!("SYSTEM: {}", dynconf_input.get("system").unwrap());
    spank_log_user!("ACCOUNT: {}", dynconf_input.get("account").unwrap());
    spank_log_user!("USER: {}", dynconf_input.get("user").unwrap());
    */

    return dynconf_input;
}
