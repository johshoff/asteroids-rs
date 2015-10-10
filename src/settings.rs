extern crate rustc_serialize;
use std::io::prelude::*;
use std::fs::File;
use self::rustc_serialize::*;

#[derive(RustcDecodable, RustcEncodable)]
pub struct Settings  {
    pub rotation_speed:      f32,
    pub drag:                f32,
    pub acceleration:        f32,
    pub print_fps:           bool,
    pub fullscreen:          bool,
    pub message_interval_ms: u64,
}

pub fn load_settings(filename: &str) -> Settings {
    let mut f = File::open(filename).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let decoded: Settings = json::decode(&s).unwrap();

    decoded
}
