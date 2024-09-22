fn main() {
    tonic_build::compile_protos("proto/profile.proto").unwrap();
}
