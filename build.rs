fn main() {
    slint_build::compile("ui/app.slint").expect("compile ui/app.slint");
    // Windows: the .exe's icon (Explorer, taskbar) and version info.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/plaza.ico").set("ProductName", "Plaza").set("FileDescription", "Plaza");
        res.compile().expect("compile Windows resources");
    }
}
