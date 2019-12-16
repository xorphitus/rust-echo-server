use std::io;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    loop {
        let mut buf = [0; 1024];
        match stream.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    println!("close connection");
                    break;
                }
                match stream.write_all(&buf[0..n]) {
                    Ok(_) => {},
                    Err(e) => panic!("{}", e),
                }
            },
            Err(e) => panic!("{}", e),
        }
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8081")?;

    for stream in listener.incoming() {
        let s = stream?;
        thread::spawn(|| {
            handle_client(s)
        });
    }

    Ok(())
}
