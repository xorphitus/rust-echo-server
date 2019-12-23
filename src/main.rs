use std::io;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use regex::Regex;
use threadpool::ThreadPool;

use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

fn handle_client(mut stream: TcpStream, tx: Sender<String>) {
    loop {
        let mut buf = [0; 1024];
        match stream.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    println!("close connection");
                    break;
                }
                stream.write_all(&buf[0..n]).unwrap();
                match stream.write_all(&buf[0..n]) {
                    Ok(_) => {
                        let l = String::from_utf8(buf[0..n].to_vec()).unwrap();
                        tx.send(l).unwrap();
                    },
                    Err(e) => panic!("{}", e),
                }
            },
            Err(e) => panic!("{}", e),
        }
    }
}

fn get_cores() -> usize {
    let path = "/proc/cpuinfo";
    let f = File::open(path).unwrap();
    let re = Regex::new(r"^processor\s+.+$").unwrap();

    let file = BufReader::new(&f);
    let mut i = 0;
    for line in file.lines() {
        let l = line.unwrap();
        if re.is_match(&l) {
            i += 1;
        }
    }
    i
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8081")?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for r in rx {
            println!("{}", r);
        }
    });

    let n_workers = get_cores();
    let pool = ThreadPool::new(n_workers);

    for stream in listener.incoming() {
        let s = stream?;
        let t = tx.clone();
        pool.execute(|| {
            handle_client(s, t);
        });
    }

    Ok(())
}
