fn main() {
    // Embed the app icon into the .exe (only matters when building for Windows).
    #[cfg(target_os = "windows")]
    embed_resource::compile("app.rc", embed_resource::NONE);
}
