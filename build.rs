use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|e| panic!("{}", e));

    if target.contains("darwin") || target.contains("ios") {
        println!("cargo:rustc-link-lib=framework=MetalKit");
    } else if target.contains("haiku") {
        cc::Build::new()
            .cpp(true)
            .include("src/native/haiku")
            .file("src/native/haiku/QuadWindow.cpp")
            .compile("shims_lib");

        println!("cargo:rustc-link-lib=be");
        println!("cargo:rustc-link-lib=game");
        println!("cargo:rustc-link-lib=GL");

        println!("cargo:rerun-if-changed=src/native/haiku/QuadWindow.cpp");
    }
}
