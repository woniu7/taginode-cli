use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    let http_request: String = buf_reader.by_ref()
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .find(|line| line.starts_with("Content-Length:"))
        .unwrap();
    let len_body = &http_request["Content-Length: ".len()..].parse::<usize>().unwrap_or(0);

    let mut buf_body = vec![0; *len_body]; 
    buf_reader.read_exact(&mut buf_body);
    println!("Request len: {len_body:#?}");
    println!("Request body: {buf_body:#?}");
}