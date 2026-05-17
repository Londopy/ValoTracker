fn main() {
    // Embed the app icon into the Windows executable so it shows up
    // in Explorer, the taskbar, and the desktop shortcut.
    // Only runs on Windows; no-op on other targets.
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("failed to embed icon via winresource");
    }
}
