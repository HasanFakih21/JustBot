use std::{fs, process::Command};

const BASE_URL: &str =
    "https://github.com/HasanFakih21/JustBot-Networks/releases/download/Networks";
const NETWORK_NAME: &str = "176-116-54-20-12-512HL-HM-Output-Buckets-320.nnue";

fn main() {
    download_netowrk();
}

fn download_netowrk() {
    let output = Command::new("curl")
        .args(["-s", "-O", "-L"])
        .arg(format!("{BASE_URL}/{NETWORK_NAME}"))
        .output()
        .expect("Error executing 'curl'!");

    if output.status.success() {
        fs::rename(NETWORK_NAME, "model.nnue").expect("Error renaming file!");
    } else {
        panic!("Error downloading network!");
    }
}
