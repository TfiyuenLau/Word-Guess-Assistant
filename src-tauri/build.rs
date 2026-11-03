fn main() {
    println!("cargo:rerun-if-changed=proto/game.proto");

    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config
        .compile_protos(&["proto/game.proto"], &["proto/"])
        .expect("Failed to compile protobuf definitions");

    tauri_build::build();
}
