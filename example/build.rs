#![feature(collections)]

use std::process::Command;
use std::env;
use std::string::String;

fn main() {
    //let out_dir = env::var("OUT_DIR").unwrap();
    let top_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let wl_prefix = match env::var("WAYLAND_PREFIX") {
        Ok(val) => val,
        Err(_) => String::from_str("/usr")
    };

    Command::new("python3").arg(&format!("{}/../scanner.py", top_dir))
        .arg(&"-o")
        .arg(&format!("{}/src/client/wayland_client_protocol.rs", top_dir))
        .arg(&format!("{}/share/wayland/wayland.xml", wl_prefix))
        .status().unwrap();

    Command::new("bindgen").args(&["-l", "wayland-client", "-builtins", "-o"])
        .arg(&format!("{}/src/client/wayland_client.rs", top_dir))
        .arg(&format!("{}/include/wayland-client.h", wl_prefix))
        .status().unwrap();
}
