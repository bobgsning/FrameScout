use std::env;
use std::path::Path;

fn main() {
    // 🌟 这一句是灵魂：强制告诉 Rust，只要 search.proto 变了，必须重新编译，绝对不能用缓存！
    // 🌟 Force cargo to re-run this build script whenever search.proto changes
    println!("cargo:rerun-if-changed=../../proto/search.proto");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let protoc_path = Path::new(&manifest_dir).join("../../proto/protoc.exe");
    
    // Set PROTOC environment variable so prost-build can find the protobuf compiler
    if protoc_path.exists() {
        env::set_var("PROTOC", &protoc_path);
    }

    // Compile the protobuf schema into Rust types
    prost_build::compile_protos(&["../../proto/search.proto"], &["../../proto/"])
        .expect("Failed to compile protobuf definitions");

    // Run Tauri's build script (handles icon generation, config validation, etc.)
    tauri_build::build();
}
