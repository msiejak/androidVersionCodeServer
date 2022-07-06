extern crate core;

use android_version_code_server::ThreadPool;
use std::fs::{read, write};
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::str::FromStr;
use serde_json::Value;
use users::{get_user_by_uid, get_current_uid};


fn main() {
    let listener = TcpListener::bind("127.0.0.1:92").unwrap();
    let pool = ThreadPool::new(4);

    listen(listener, pool)

}

fn listen(listener: TcpListener, pool: ThreadPool) {
    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }
    listen(listener, pool);
}

fn find_between<'a>(text: &'a String, start: &str, end: &str) -> &'a str {
    let start_bytes = text.find(start).unwrap_or(0) + start.len();
    let end_bytes = text.find(end).unwrap_or(text.len());
    &text[start_bytes..end_bytes]
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let get = b"GET /get";
    let set = b"GET /set";
    let buffer_text = String::from(String::from_utf8_lossy(&buffer));
    let user = get_user_by_uid(get_current_uid()).unwrap();
    let file_path = format!("/home/{}/version_codes.json", user.name().to_string_lossy());
    let json_file = read(&file_path).unwrap();

    let (status_line, output) = if buffer.starts_with(get) {
        let app_name = find_between(&buffer_text, "package=", " HTTP/1.1");
        let json: Value = serde_json::from_slice(&json_file).unwrap();
        println!("{}", app_name);
        let version = json[app_name].as_i64().unwrap();
        let final_str = version.to_string();

        ("HTTP/1.1 200 OK", final_str)
    } else if buffer.starts_with(set) {
        let version_code = find_between(&buffer_text, "&versionCode=", " HTTP/1.1");
        let app_name = find_between(&buffer_text, "package=", "&versionCode");
        println!("{}", version_code);
        let mut json: Value = serde_json::from_slice(&json_file).unwrap();
        println!("updated {} versionCode", app_name);
        json[app_name] = Value::from(i64::from_str(version_code).unwrap());
        write(&file_path, serde_json::to_string_pretty(&json).unwrap()).expect("Error writing file");
        ("HTTP/1.1 200 OK", String::from("Set new value"))
    } else {
        ("HTTP/1.1 404 NOT FOUND", String::from("Invalid Request"))
    };

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        output.len(),
        output
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
