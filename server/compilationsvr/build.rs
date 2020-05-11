use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;
fn main() {
	tonic_build::configure()
		.build_server(true)
		.compile(&["proto/rpc.proto"], &["proto/", "../proto"])
		.unwrap();
	let current_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
	let comp_dir = current_dir.join("./compilation/");
	let out_dir = env::var("OUT_DIR").unwrap();

	println!("{}", out_dir);
	Command::new("cp")
		.arg("-r")
		.arg(comp_dir.to_str().unwrap())
		.arg(out_dir.to_string())
		.spawn()
		.expect("copy compilation script failed");
}
