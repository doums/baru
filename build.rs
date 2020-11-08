fn main() {
    let libnl_data_dst = cmake::build("libnl_data");
    let libsound_dst = cmake::build("libsound");
    println!(
        "cargo:rustc-link-search=native={}/lib",
        libnl_data_dst.display()
    );
    println!(
        "cargo:rustc-link-search=native={}/lib",
        libsound_dst.display()
    );
    println!("cargo:rustc-link-lib=static=nl_data");
    println!("cargo:rustc-link-lib=dylib=nl-3");
    println!("cargo:rustc-link-lib=dylib=nl-genl-3");
    println!("cargo:rustc-link-lib=dylib=nl-route-3");
    println!("cargo:rustc-link-lib=dylib=pulse");
}
