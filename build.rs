use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Check the target architecture
    let target = env::var("TARGET").unwrap_or_default();

    // If the target is `wasm32-unknown-unknown`, set the crate type to `cdylib`
    if target == "wasm32-unknown-unknown" {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let lib_section = r#"
[lib]
crate-type = ["cdylib"]
"#;
        let cargo_toml_path = Path::new(&manifest_dir).join("Cargo.toml");
        let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();

        if !cargo_toml.contains("[lib]") {
            cargo_toml.push_str(lib_section);
            fs::write(cargo_toml_path, cargo_toml).unwrap();
        }
    }
}
