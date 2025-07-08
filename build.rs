// build.rs
#[cfg(windows)]
fn main() {
    use winres::WindowsResource;
    
    WindowsResource::new()
        .set_icon("icon.ico")
        .set("ProductName", "Browser Backup")
        .set("FileDescription", "Browser Favoriten Backup Tool")
        .set("LegalCopyright", "GAV 2025")
        .compile()
        .unwrap();
}

#[cfg(not(windows))]
fn main() {}