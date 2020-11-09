fn main() {
    let netlink_dst = cmake::build("lib/netlink");
    let audio_dst = cmake::build("lib/audio");
    println!(
        "cargo:rustc-link-search=native={}/lib",
        netlink_dst.display()
    );
    println!("cargo:rustc-link-search=native={}/lib", audio_dst.display());
    println!("cargo:rustc-link-lib=dylib=nl-3");
    println!("cargo:rustc-link-lib=dylib=nl-genl-3");
    println!("cargo:rustc-link-lib=dylib=nl-route-3");
    println!("cargo:rustc-link-lib=dylib=pulse");
}
