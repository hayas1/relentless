fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos: Vec<_> = std::fs::read_dir("proto/")?
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.is_file() && matches!(p.extension()?.to_str()?, "proto")).then_some(p)
        })
        .collect();

    tonic_prost_build::configure()
        // .compile_well_known_types(true)
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("file_descriptor.bin"))
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&protos, &["proto/".into()])?;

    // https://github.com/hyperium/tonic/issues/2526
    // https://www.au92.com/post/tonic-version-update/#:~:text=proto%E6%96%87%E4%BB%B6%E4%B8%AD%E4%BD%BF%E7%94%A8%E4%BA%86,%E4%B8%80%E7%AF%87%20%E4%B8%8B%E4%B8%80%E7%AF%87
    for entry in std::fs::read_dir(std::env::var("OUT_DIR")?)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            let content = content.replace("super::i64", "i64").replace("super::u64", "u64");
            std::fs::write(&path, content)?;
        }
    }

    Ok(())
}
