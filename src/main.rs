extern crate chrono;
#[macro_use]
extern crate lazy_static;

#[cfg(target_os = "linux")]
use std::fs::File;
use std::io;
#[cfg(target_os = "linux")]
use std::io::BufRead;
#[cfg(target_os = "linux")]
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;

use chrono::{DateTime, Local};
use failure::Error;
use regex::Regex;
use threadpool::ThreadPool;

const IP_ADDRESS: &str = "127.0.0.1";
const PORT: u16 = 8081;
const BUF_SIZE: usize = 1024;
const DEFAULT_WORKER_NUM: usize = 1;

#[cfg(target_os = "linux")]
lazy_static! {
    static ref CPU_RE: Regex = Regex::new(r"^processor\s+.+$").unwrap();
}

#[cfg(target_os = "macos")]
lazy_static! {
    static ref CPU_RE: Regex = Regex::new(r"\s*Number of Cores:\s*(\d+)\s*").unwrap();
}

fn handle_client(mut stream: TcpStream, log_tx: Sender<String>) -> Result<(), Error> {
    loop {
        let mut buf = [0; BUF_SIZE];
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }

        let msg = &&buf[0..n];
        stream.write_all(msg)?;

        let msg = String::from_utf8(msg.to_vec())?;
        log_tx.send(msg)?;
    }
}

#[cfg(target_os = "linux")]
fn get_cores() -> Result<usize, Error> {
    let path = "/proc/cpuinfo";
    let f = File::open(path)?;

    let file = BufReader::new(&f);
    let mut i = 0;
    for line in file.lines() {
        if CPU_RE.is_match(&line?) {
            i += 1;
        }
    }
    Ok(i)
}

#[cfg(target_os = "macos")]
fn get_cores() -> Result<usize, Error> {
    let sys_prof = Command::new("system_profiler")
        .arg("SPHardwareDataType")
        .output()?;
    let sys_prof = String::from_utf8(sys_prof.stdout)?;

    let num = CPU_RE.captures(&sys_prof)
        .and_then(|cap| cap.get(1))
        .ok_or_else(|| failure::err_msg("couldn't find a CPU core number from `system_profiler SPHardwareDataType`"))?;

    let num = num.as_str().parse::<usize>()?;
    Ok(num)
}

fn main() -> io::Result<()> {
    // logger
    let (log_tx, log_rx) = mpsc::channel();
    thread::spawn(move || {
        for r in log_rx {
            let time: DateTime<Local> = Local::now();
            println!("{}\t{}", time, r);
        }
    });

    let worker_num = get_cores().unwrap_or_else(|e| {
        eprintln!("failed to get CPU cores: {}", e);
        DEFAULT_WORKER_NUM
    });
    let pool = ThreadPool::new(worker_num);
    eprintln!("{} workers were set", worker_num);

    let listener = TcpListener::bind(format!("{}:{}", IP_ADDRESS, PORT))?;
    for stream in listener.incoming() {
        let s = stream?;
        let tx = log_tx.clone();
        pool.execute(|| {
            handle_client(s, tx)
                .and_then(|_| {
                    eprintln!("close connection");
                    Ok(())
                })
                .unwrap_or_else(|e| eprintln!("an error occurred: {}" , e));
        });
    }

    Ok(())
}
