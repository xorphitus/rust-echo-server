use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;

use failure::Error;
use regex::Regex;
use threadpool::ThreadPool;

const BUF_SIZE: usize = 1024;
const DEFAULT_WORKER_NUM: usize = 1;

fn handle_client(mut stream: TcpStream, log_tx: &Sender<String>) -> Result<(), Error> {
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

fn get_cores() -> Result<usize, Error> {
    let path = "/proc/cpuinfo";
    let f = File::open(path)?;
    // Using `unwrap`: this is reasonable.
    // Since this program should be terminated ASAP when the regexp is incorrect.
    let re = Regex::new(r"^processor\s+.+$").unwrap();

    let file = BufReader::new(&f);
    let mut i = 0;
    for line in file.lines() {
        if re.is_match(&line?) {
            i += 1;
        }
    }
    Ok(i)
}

fn main() -> io::Result<()> {
    // logger
    let (log_tx, log_rx) = mpsc::channel();
    thread::spawn(move || {
        for r in log_rx {
            println!("{}", r);
        }
    });

    let worker_num = get_cores().unwrap_or_else(|e| {
        eprintln!("failed to get CPU cores: {}", e);
        DEFAULT_WORKER_NUM
    });
    let pool = ThreadPool::new(worker_num);

    let listener = TcpListener::bind("127.0.0.1:8081")?;
    for stream in listener.incoming() {
        let s = stream?;
        let tx = log_tx.clone();
        pool.execute(move || {
            handle_client(s, &tx)
                .and_then(|_| {
                    eprintln!("close connection");
                    Ok(())
                })
                .unwrap_or_else(|e| eprintln!("an error occurred: {}" , e));
        });
    }

    Ok(())
}
