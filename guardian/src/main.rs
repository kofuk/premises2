use log::info;
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::process::{Command, Stdio};

fn agree_eula() -> std::io::Result<()> {
    create_dir_all("/tmp/m")?;

    let mut out = File::create("/tmp/m/eula.txt")?;
    write!(out, "eula=true")?;

    Ok(())
}

fn main() {
    env_logger::init();

    agree_eula().unwrap();

    loop {
        let result = Command::new("java")
            .args(["-jar", "/tmp/server.jar", "nogui"])
            .current_dir("/tmp/m")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        if result.success() {
            break;
        }

        info!(
            "Minecraft server exitted abnormally (code: {}). Restarting...",
            result.code().unwrap()
        );
    }
}
