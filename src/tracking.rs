use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{Write, pipe};
use std::process::{Command, Stdio};

use slurm_spank::SpankHandle;

use crate::{SpankSkyBox, plugin_string, spank_getenv};

#[derive(Serialize, Deserialize, Debug)]
struct Record {
    account: String,
    command: String,
    context: String,
    engine: String,
    environment: Environment,
    image: String,
    jobid: usize,
    nnodes: usize,
    nodelist: String,
    stepid: usize,
    user: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Environment {
    name: String,
    content: String,
}

pub(crate) fn track_usage(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<(), Box<dyn Error>> {
    if !ssb.config.tracking_enabled {
        return Ok(());
    }
    let data = track_slurmstepd_data_collect(ssb, spank)?;
    track_send(data, ssb)?;

    Ok(())
}

pub(crate) fn track_slurmstepd_data_collect(
    ssb: &mut SpankSkyBox,
    spank: &mut SpankHandle,
) -> Result<String, Box<dyn Error>> {
    let edf = match &ssb.edf {
        Some(o) => o,
        None => {
            return Err(plugin_string("couldn't find edf").into());
        }
    };

    let job = match &ssb.job {
        Some(o) => o,
        None => {
            return Err(plugin_string("couldn't find job").into());
        }
    };

    let jobid: usize = job.jobid.try_into()?;
    let stepid: usize = job.stepid.try_into()?;

    let env_name = match &ssb.args.edf {
        Some(o) => o.to_string(),
        None => {
            return Err(plugin_string("couldn't find args.edf").into());
        }
    };
    let env_content = spank_getenv(spank, "SLURM_EDF_EXPANDED");

    let environment = Environment {
        name: env_name,
        content: env_content,
    };

    let account = spank_getenv(spank, "SLURM_JOB_ACCOUNT");

    let argv = match spank.job_argv() {
        Ok(a) => a,
        Err(_) => {
            return Err(plugin_string("couldn't read job_argv").into());
        }
    };

    let mut command = String::from("");
    for i in argv.iter() {
        if command != "" {
            command.push_str(" ");
        }
        command.push_str(i);
    }

    let nodelist = spank_getenv(spank, "SLURM_NODELIST");
    let user = spank_getenv(spank, "SLURM_JOB_USER");

    let record = Record {
        account: account,
        command: command,
        context: String::from("slurmstepd"),
        engine: String::from("podman"),
        environment: environment,
        image: edf.image.clone(),
        jobid: jobid,
        nnodes: 0,
        nodelist: nodelist,
        stepid: stepid,
        user: user,
    };

    let s = serde_json::to_string(&record)?;
    Ok(s)
}

pub(crate) fn track_send(data: String, ssb: &mut SpankSkyBox) -> Result<(), Box<dyn Error>> {
    let tracking_tool = &ssb.config.tracking_tool;
    let (input_reader, mut input_writer) = pipe()?;

    let mut child = Command::new(tracking_tool)
        .stdin(input_reader)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    input_writer.write_all(data.as_bytes())?;
    drop(input_writer);
    child.wait()?;

    Ok(())
}
