extern crate protoc_rust;
use protoc_rust::Customize;
use std::env;
use std::path::PathBuf;

fn main() {
	let current_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
	let proto_dir = current_dir.join("..").join("interface");
	let mut proto = String::from(proto_dir.to_str().unwrap());
	proto.push_str("/judgesrv.proto");
	protoc_rust::run(protoc_rust::Args {
		out_dir: "src/interface/",
		input: &[&proto],
		includes: &[proto_dir.to_str().unwrap()],
		customize: Customize {
			..Default::default()
		},
	})
	.expect("protoc");
}
