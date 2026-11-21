// Embeds data/patrician2.p2d into the patcher. Without it the patcher is built with an empty
// patch (needed to run the `make` command that creates the patch file in the first place).
use std::{env, fs, path::PathBuf};

const PATCH: &str = "data/patrician2.p2d";

fn main() {
    println!("cargo:rerun-if-changed={PATCH}");
    println!("cargo:rerun-if-changed=data");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("embedded.p2d");
    let bytes = fs::read(PATCH).unwrap_or_default();
    fs::write(out, bytes).expect("write embedded patch");
}
