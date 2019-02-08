extern crate bindgen;

use std::env;
use std::path::Path;
use std::path::PathBuf;

static SPDK_INCLUDE_DIR: &'static str = "/usr/local/include";

fn main() {
    let spdk_include_path = env::var("SPDK_INCLUDE").unwrap_or(SPDK_INCLUDE_DIR.to_string());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-lib=spdk");
    println!("cargo:rustc-link-search=native=/usr/local/lib");

    let mut codegen_config = bindgen::CodegenConfig::empty();
        codegen_config.set(bindgen::CodegenConfig::FUNCTIONS, true);
        codegen_config.set(bindgen::CodegenConfig::TYPES, true);
        codegen_config.set(bindgen::CodegenConfig::CONSTRUCTORS, true);
    codegen_config.set(bindgen::CodegenConfig::METHODS, true);
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .derive_default(true)
        .header("wrapper.h")
        .with_codegen_config(codegen_config)
        .generate_inline_functions(false)
            // If there are linking errors and the generated bindings have weird looking
            // #link_names (that start with \u{1}), the make sure to flip that to false.
        .trust_clang_mangling(false)
        .rustfmt_bindings(true)
        .rustfmt_configuration_file(Some(PathBuf::from("../rustfmt.toml")))
        .layout_tests(false)
        .ctypes_prefix("libc")
        // The input header we would like to generate
        // bindings for.
        .derive_default(true)
        .blacklist_type("IPPORT_.*")   // https://github.com/rust-lang-nursery/rust-bindgen/issues/687
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .opaque_type("spdk_nvme_feat_async_event_configuration") // https://github.com/rust-lang-nursery/rust-bindgen/issues/687
        .opaque_type("spdk_nvme_feat_async_event_configuration__bindgen_ty_1")
        // If there are linking errors and the generated bindings have weird looking
        // #link_names (that start with \u{1}), the make sure to flip that to false.
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
