mod rcon;

use crossbeam_channel as channel;
use log::info;
use monitor::start_monitoring;
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::process::{Command, Stdio};
use std::{thread, time};

fn agree_eula() -> std::io::Result<()> {
    create_dir_all("/tmp/m")?;

    let mut out = File::create("/tmp/m/eula.txt")?;
    write!(out, "eula=true")?;

    Ok(())
}

mod monitor {
    use crossbeam_channel::{self as channel, select, Receiver, Sender};
    use log::debug;
    use std::{net::TcpStream, time};

    use crate::rcon::RconClient;

    #[derive(Debug)]
    pub enum Event {
        Online,
        Offline,
    }

    fn try_execute_command(cmd: &str) -> Option<String> {
        let transport = match TcpStream::connect(("localhost", 25575)) {
            Ok(transport) => transport,
            Err(err) => {
                debug!("Failed to connect to RCON: {}", err);
                return None;
            }
        };

        let mut rcon = RconClient::new(transport, false);
        rcon.authenticate("x").unwrap();
        rcon.execute(cmd).ok()
    }

    fn handle_event(event: &str) {
        try_execute_command(event);
    }

    fn do_health_check(out_ev: &Sender<Event>) {
        match try_execute_command("list") {
            Some(_) => out_ev.send(Event::Online).unwrap(),
            None => out_ev.send(Event::Offline).unwrap(),
        };
    }

    pub fn start_monitoring(in_ev: Receiver<&str>, out_ev: Sender<Event>) {
        let monitor_tick = channel::tick(time::Duration::from_secs(5));
        loop {
            select! {
                recv(in_ev) -> job => {
                    if let Ok(job) = job {
                        handle_event(job);
                    }
                }
                recv(monitor_tick) -> _ => {
                    do_health_check(&out_ev);
                }
            };
        }
    }
}

fn main() {
    env_logger::init();

    let (job_tx, job_rx) = channel::unbounded();
    let (event_tx, event_rx) = channel::unbounded::<monitor::Event>();

    // debug
    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(20));
        job_tx.send("op hogehogefugafuga").unwrap();
        thread::sleep(time::Duration::from_secs(2));
        job_tx.send("stop").unwrap();

        loop {
            thread::sleep(time::Duration::from_secs(20));
        }
    });
    thread::spawn(move || loop {
        if let Ok(ev) = event_rx.recv() {
            println!("{:?}", ev);
        };
    });

    let job_thread = thread::spawn(move || start_monitoring(job_rx.clone(), event_tx.clone()));

    agree_eula().unwrap();

    loop {
        let result = Command::new("/tmp/mcserver-mock")
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

    job_thread.join().unwrap();
}
