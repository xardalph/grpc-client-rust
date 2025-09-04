use tonic_build;
fn main() {
    tonic_build::configure()
        .file_descriptor_set_path("helloworld_descriptor.bin")
        .compile(&["proto/helloworld.proto"], &["proto"])
        .unwrap();
}
