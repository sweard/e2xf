fn main() {
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent-light".into()),
    )
    .expect("Failed to compile the Slint interface");

    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("windows/runner/resources/app_icon.ico")
        .set("ProductName", "e2xf")
        .set("FileDescription", "Excel to Android strings.xml")
        .compile()
        .expect("Failed to embed Windows application resources");
}
