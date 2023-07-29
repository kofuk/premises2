use chrono::{DateTime, Local};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use std::time::{Duration, SystemTime};

const LOG_INFO: &'static str = "INFO";
const LOG_WARN: &'static str = "WARN";
const LOG_ERROR: &'static str = "ERROR";

fn mc_log(topic: &str, level: &str, message: &str) {
    std::thread::sleep(Duration::from_millis((rand::random::<u8>() as u64) << 2));

    let time: DateTime<Local> = SystemTime::now().into();
    let time_str = time.format("%T");
    println!("[{time_str}] [{topic}/{level}]: {message}")
}

fn check_for_server_properties() {
    if Path::new("server.properties").exists() {
        let content = {
            let mut content = String::new();
            File::open("server.properties")
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            content
        };

        // If there's invalid lines, exit the server (for convenience).
        match content
            .split("\n")
            .filter(|line| line.bytes().into_iter().nth(0).unwrap_or(b'#') != b'#')
            .find(|line| !line.contains('='))
        {
            Some(line) => {
                eprintln!("Debug: {}: Invalid line", line.replace("\r", ""));
                process::exit(1);
            }
            None => (),
        };

        return;
    }

    mc_log(
        "ServerMain",
        LOG_ERROR,
        "Failed to load properties from file: server.properties",
    );
    println!(
        "{}",
        "java.nio.file.NoSuchFileException: server.properties
	at sun.nio.fs.UnixException.translateToIOException(UnixException.java:92) ~[?:?]
	at sun.nio.fs.UnixException.rethrowAsIOException(UnixException.java:106) ~[?:?]
	at sun.nio.fs.UnixException.rethrowAsIOException(UnixException.java:111) ~[?:?]
	at sun.nio.fs.UnixFileSystemProvider.newByteChannel(UnixFileSystemProvider.java:261) ~[?:?]
	at java.nio.file.Files.newByteChannel(Files.java:379) ~[?:?]
	at java.nio.file.Files.newByteChannel(Files.java:431) ~[?:?]
	at java.nio.file.spi.FileSystemProvider.newInputStream(FileSystemProvider.java:422) ~[?:?]
	at java.nio.file.Files.newInputStream(Files.java:159) ~[?:?]
	at ahi.b(SourceFile:62) ~[server-1.20.1.jar:?]
	at ahf.a(SourceFile:137) ~[server-1.20.1.jar:?]
	at ahg.<init>(SourceFile:12) ~[server-1.20.1.jar:?]
	at net.minecraft.server.Main.main(SourceFile:115) ~[server-1.20.1.jar:?]
	at net.minecraft.bundler.Main.lambda$run$0(Main.java:54) ~[?:?]
	at java.lang.Thread.run(Thread.java:1623) ~[?:?]"
    );

    File::create("server.properties")
        .unwrap()
        .write(include_bytes!("server-properties-skeleton.txt"))
        .unwrap();
}

fn check_for_eula_txt() {
    let signed = match File::open("eula.txt") {
        Ok(mut file) => {
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            content
                .split("\n")
                .find(|line| line.replace("\r", "") == "eula=true")
                .is_some()
        }
        Err(_) => false,
    };

    if signed {
        return;
    }

    mc_log("ServerMain", LOG_WARN, "Failed to load eula.txt");
    mc_log(
        "ServerMain",
        LOG_INFO,
        "You need to agree to the EULA in order to run the server. Go to eula.txt for more info.",
    );

    File::create("eula.txt")
        .unwrap()
        .write(b"eula=false\n")
        .unwrap();

    process::exit(0);
}

fn prepare_server() {
    r#"Starting mock minecraft server version 0.10.0 (emulating 1.20.1)
Loading properties
Default game type: SURVIVAL
Generating keypair
Starting Minecraft server on *:25565
Using epoll channel type
Preparing level "world"
Preparing start region for dimension minecraft:overworld"#
        .split("\n")
        .for_each(|line| mc_log("Server thread", LOG_INFO, line));

    let mut total = 0;
    while total <= 100 {
        mc_log(
            &format!("Worker-Main-{}", rand::random::<u8>() % 4 + 1),
            LOG_INFO,
            &format!("Preparing spawn area: {total}%"),
        );

        total += rand::random::<u8>() % 20;
    }

    mc_log("Server thread", LOG_INFO, "Time elapsed 14083 ms");
    mc_log(
        "Server thread",
        LOG_INFO,
        "Done (16.760s)! For help, type \"help\"",
    );
}

fn stop_server() {
    "Stopping the server
Stopping server
Saving players
Saving worlds
Saving chunks for level 'ServerLevel[world]'/minecraft:overworld
Saving chunks for level 'ServerLevel[world]'/minecraft:the_end
Saving chunks for level 'ServerLevel[world]'/minecraft:the_nether
ThreadedAnvilChunkStorage (world): All chunks are saved
ThreadedAnvilChunkStorage (DIM1): All chunks are saved
ThreadedAnvilChunkStorage (DIM-1): All chunks are saved
ThreadedAnvilChunkStorage: All dimensions are saved"
        .split("\n")
        .for_each(|line| mc_log("Server thread", LOG_INFO, line));
}

fn main() {
    println!("Starting net.minecraft.server.Main");

    check_for_server_properties();
    check_for_eula_txt();

    prepare_server();

    stop_server();
}
