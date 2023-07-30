use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process;
use std::time::{Duration, SystemTime};

const LOG_INFO: &str = "INFO";
const LOG_WARN: &str = "WARN";
const LOG_ERROR: &str = "ERROR";

fn mc_log(topic: &str, level: &str, message: &str) {
    std::thread::sleep(Duration::from_millis((rand::random::<u8>() as u64) << 2));

    let time: DateTime<Local> = SystemTime::now().into();
    let time_str = time.format("%T");
    println!("[{time_str}] [{topic}/{level}]: {message}")
}

fn load_server_properties() -> HashMap<String, String> {
    if Path::new("server.properties").exists() {
        let content = {
            let mut content = String::new();
            File::open("server.properties")
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            content
        };

        let server_props = content
            .split('\n')
            .filter(|line| line.bytes().next().unwrap_or(b'#') != b'#')
            .filter_map(|line| {
                line.find('=')
                    .map(|pos| (line[..pos].to_string(), line[pos + 1..].to_string()))
            })
            .collect();

        return server_props;
    }

    mc_log(
        "ServerMain",
        LOG_ERROR,
        "Failed to load properties from file: server.properties",
    );
    println!(
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
        .write_all(include_bytes!("server-properties-skeleton.txt"))
        .unwrap();

    load_server_properties()
}

fn check_for_eula_txt() {
    let signed = match File::open("eula.txt") {
        Ok(mut file) => {
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            content
                .split('\n')
                .any(|line| line.replace('\r', "") == "eula=true")
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
        .write_all(b"eula=false\n")
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
        .split('\n')
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
Thread RCON Listener stopped
ThreadedAnvilChunkStorage: All dimensions are saved"
        .split('\n')
        .for_each(|line| mc_log("Server thread", LOG_INFO, line));
}

fn get_rcon_settings(server_props: &HashMap<String, String>) -> (bool, String, u16) {
    match server_props.get("enable-rcon") {
        Some(v) => {
            if v == "true" {
                (
                    true,
                    server_props
                        .get("rcon.password")
                        .unwrap_or(&"".to_string())
                        .to_owned(),
                    server_props
                        .get("rcon.port")
                        .map(|v| v.parse::<u16>().unwrap())
                        .unwrap_or(25575),
                )
            } else {
                (false, "".to_string(), 0)
            }
        }
        None => (false, "".to_string(), 0),
    }
}

mod rcon {
    use super::{mc_log, LOG_INFO};
    use std::{
        io::{BufWriter, Read, Write},
        net::TcpListener,
    };

    #[derive(Debug)]
    pub struct Packet {
        req_id: i32,
        pack_type: i32,
        payload: String,
    }

    impl Packet {
        fn new(req_id: i32, pack_type: i32, payload: String) -> Self {
            Self {
                req_id,
                pack_type,
                payload,
            }
        }

        fn read_from_stream<R>(strm: &mut R) -> Option<Packet>
        where
            R: Read,
        {
            let mut buf = [0u8; 4];
            if strm.read_exact(&mut buf).is_err() {
                return None;
            }

            let length = i32::from_le_bytes(buf);
            assert!(length >= 9);

            strm.read_exact(&mut buf).unwrap();
            let req_id = i32::from_le_bytes(buf);

            strm.read_exact(&mut buf).unwrap();
            let pack_type = i32::from_le_bytes(buf);

            let payload_len = length - 4 - 4;
            let mut payload_bytes = vec![0u8; payload_len as usize];
            strm.read_exact(&mut payload_bytes).unwrap();

            let payload = String::from_utf8(
                payload_bytes
                    .into_iter()
                    .take_while(|b| b != &b'\0')
                    .collect(),
            )
            .unwrap();

            Some(Packet {
                req_id,
                pack_type,
                payload,
            })
        }

        fn send_to_stream<W>(&self, strm: &mut W)
        where
            W: Write,
        {
            let len = (4 + 4 + 2 + self.payload.as_bytes().len()) as i32;
            let mut writer = BufWriter::new(strm);
            writer.write_all(&len.to_le_bytes()).unwrap();
            writer.write_all(&self.req_id.to_le_bytes()).unwrap();
            writer.write_all(&self.pack_type.to_le_bytes()).unwrap();
            writer.write_all(self.payload.as_bytes()).unwrap();
            writer.write_all(b"\0\0").unwrap();
        }
    }

    enum Status {
        Disconnect,
        Exit,
    }

    fn do_command_loop<S>(strm: &mut S) -> Status
    where
        S: Read + Write,
    {
        loop {
            match Packet::read_from_stream(strm) {
                Some(Packet {
                    req_id,
                    pack_type: 2,
                    payload,
                }) => match *payload.split(' ').collect::<Vec<&str>>().as_slice() {
                    ["stop"] => {
                        Packet::new(req_id, 0, "Stopping the server".to_string())
                            .send_to_stream(strm);
                        break Status::Exit;
                    }
                    ["whitelist", "add", user] => {
                        Packet::new(req_id, 0, format!("Added {user} to the whitelist"))
                            .send_to_stream(strm);
                    }
                    ["op", user] => {
                        Packet::new(req_id, 0, format!("Made {user} a server operator"))
                            .send_to_stream(strm);
                    }
                    _ => {
                        Packet::new(
                            req_id,
                            0,
                            format!(
                                "Unknown or incomplete command, see below for error{}<--[HERE]",
                                payload
                            ),
                        )
                        .send_to_stream(strm);
                    }
                },
                Some(Packet {
                    req_id, pack_type, ..
                }) => {
                    Packet::new(req_id, 0, format!("Unknown request {:x}", pack_type));
                }
                None => break Status::Disconnect,
            };
        }
    }

    pub fn listen(passwd: String, port: u16) {
        let listener = TcpListener::bind(("0.0.0.0", port)).unwrap();

        mc_log(
            "Server thread",
            LOG_INFO,
            &format!("RCON running on 0.0.0.0:{port}"),
        );

        loop {
            let (mut strm, addr) = listener.accept().unwrap();
            mc_log(
                "RCON Listener #1",
                LOG_INFO,
                &format!("Thread RCON Client /{} started", addr.ip()),
            );

            if let Some(Packet {
                req_id,
                pack_type: 3,
                payload,
            }) = Packet::read_from_stream(&mut strm)
            {
                if payload == passwd {
                    Packet::new(req_id, 2, "".to_string()).send_to_stream(&mut strm);

                    if let Status::Exit = do_command_loop(&mut strm) {
                        break;
                    };
                } else {
                    Packet::new(-1, 2, "authentication failed".to_string())
                        .send_to_stream(&mut strm);
                }
            } else {
                Packet::new(-1, 2, "authentication required".to_string()).send_to_stream(&mut strm);
            }

            mc_log(
                &format!("RCON Client {} #1", addr.ip()),
                LOG_INFO,
                &format!("Thread RCON Client /{} shutting down", addr.ip()),
            );
        }

        mc_log("Server thread", LOG_INFO, "[Rcon: Stopping the server]");
    }
}

fn main() {
    println!("Starting net.minecraft.server.Main");

    let server_props = load_server_properties();
    check_for_eula_txt();

    let (enable_rcon, rcon_passwd, rcon_port) = get_rcon_settings(&server_props);
    if !enable_rcon {
        eprintln!("Debug: You must enable rcon to use this mock server");
        process::exit(1);
    } else if enable_rcon && rcon_passwd.is_empty() {
        eprintln!("Debug: rcon cannot be enabled with empty password");
        process::exit(1);
    }

    prepare_server();

    rcon::listen(rcon_passwd, rcon_port);

    stop_server();
}
