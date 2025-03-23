#[cfg(feature = "native")]
#[path = "build/native.rs"]
mod native;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/native.rs");
    println!("cargo:rerun-if-changed=parallel-rdp/bridge.cpp");

    #[cfg(feature = "native")]
    native::build_renderer();
}
