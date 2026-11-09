#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    winresource::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .set("ProductName", "IWE")
        .set("FileDescription", "IWE")
        .compile()
        .expect("compile the Windows application icon");
}

#[cfg(not(windows))]
fn main() {}
