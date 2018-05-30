extern crate serde_json;

use std::process::Command;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

use self::serde_json::{Value, Error};

pub fn run(tx_name: &str) -> String {
    let current_dir = env::current_dir().unwrap();
    let current_dir = current_dir.as_path();

    let output = Command::new("./compile.sh")
        .current_dir(current_dir.join("ctest"))
        .output();
    assert!(output.is_ok());

    let output = Command::new("./test")
        .current_dir(current_dir.join("ctest"))
        .arg(tx_name)
        .output();
    assert!(output.is_ok());

    let file_path = current_dir.join("ctest").join("output").join(tx_name);
    let file = File::open(file_path);
    assert!(file.is_ok());

    let mut buf_reader = BufReader::new(file.unwrap());
    let mut contents = String::new();
    let res = buf_reader.read_to_string(&mut contents);
    assert!(res.is_ok());

    contents
}

pub fn read_inputs(tx_name: &str) -> Result<Value, Error> {
    let current_dir = env::current_dir().unwrap();
    let current_dir = current_dir.as_path();

    let file_path = current_dir.join("ctest").join("inputs").join(tx_name.to_owned() + ".json");
    let file = File::open(file_path);
    assert!(file.is_ok());

    let mut buf_reader = BufReader::new(file.unwrap());
    let mut contents = String::new();
    let res = buf_reader.read_to_string(&mut contents);
    assert!(res.is_ok());

    let v: Value = serde_json::from_str(&contents)?;
    Ok(v)
}
