
use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let top_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    Command::new("python3").arg(&format!("{}/../scanner.py", top_dir))
        .arg(&"-o")
        .arg(&format!("{}/src/wayland_client_protocol.rs", top_dir))
        .arg(&"/usr/local/share/wayland/wayland.xml")
        .status().unwrap();

    Command::new("bindgen").args(&["-builtins", "-o"])
        .arg(&format!("{}/src/wayland_client.rs", top_dir))
        .arg(&"/usr/local/include/wayland-client.h")
        .status().unwrap();
}
