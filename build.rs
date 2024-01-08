use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|e| panic!("{}", e));

    if target.contains("darwin") || target.contains("ios") {
        println!("cargo:rustc-link-lib=framework=MetalKit");
    }
    
    let prefix = env::var("PREFIX");
    if let Ok(prefix) = prefix {
        println!("prefix: {prefix}");
        if prefix.contains("com.termux") {
            println!("cargo:rustc-cfg=termux");
        }
    }
}
