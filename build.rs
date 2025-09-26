use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("outdir is : {:?}", out_dir.to_str());
    let binding = out_dir.clone();
    let data = binding.to_str().clone().unwrap_or("empty").as_bytes();
    fs::write("/tmp/foo", data).expect("Should be able to write to `/foo/tmp`");

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("helloworld_descriptor.bin"))
        .compile_protos(&["proto/helloworld.proto"], &["proto"])
        .unwrap();

    Ok(())
}
