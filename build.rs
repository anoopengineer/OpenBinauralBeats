fn main() {
    // Embed the icon and version info into the Windows executable.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("ProductName", "OpenBinauralBeats")
            .set("FileDescription", "OpenBinauralBeats")
            .set(
                "LegalCopyright",
                "Copyright (c) 2026 Anoop Kunjuraman. MIT License.",
            );
        res.compile().expect("failed to embed Windows resources");
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
