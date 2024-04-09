use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

const PROTOS: &[&str] = &["src/services/file_transfer/schema/provider.v1.proto"];

fn main() {
    generate_cargo_keys();

    // Tell Cargo to rerun this build script whenever the proto files change.
    PROTOS.iter().for_each(|proto| {
        println!("cargo:rerun-if-changed={}", proto);
    });

    rerun_if_git_head_changed();

    prost_build::compile_protos(PROTOS, &["src/services/file_transfer/schema"]).unwrap();
}
