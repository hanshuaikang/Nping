fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Specify the path to the lib files
        println!("cargo:rustc-link-search=native=./libraries");
        // Specify the name of the library to link, without the `lib` prefix and `.lib` suffix
        println!("cargo:rustc-link-lib=dylib=Packet");
    }
}