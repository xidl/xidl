fn main() {
    #[cfg(feature = "bindgen")]
    {
        use std::env;
        use std::fs;
        use std::path::PathBuf;
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
        let include_dir = manifest_dir.join("include");
        fs::create_dir_all(&include_dir).expect("create include dir");

        let out_path = include_dir.join("xidl_xcdr.h");
        let config = manifest_dir.join("cbindgen.toml");

        cbindgen::Builder::new()
            .with_crate(manifest_dir)
            .with_config(cbindgen::Config::from_file(config).expect("cbindgen config"))
            .generate()
            .expect("generate bindings")
            .write_to_file(out_path);
    }
}
