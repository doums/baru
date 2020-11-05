fn main() {
    let dst = cmake::build("libnl_data");
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=nl_data");
    println!("cargo:rustc-link-lib=dylib=nl-3");
    println!("cargo:rustc-link-lib=dylib=nl-genl-3");
    println!("cargo:rustc-link-lib=dylib=nl-route-3");
}
