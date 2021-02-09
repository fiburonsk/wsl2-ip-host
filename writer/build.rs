fn main() {
    #[cfg(windows)]
    win::run();
}

mod win {
    extern crate embed_resource;
    use winres::WindowsResource;

    pub fn run() {
        embed_resource::compile("wsl2-ip-host-writer.exe.rc");
        WindowsResource::new()
            .set_icon("./../resources/icon.ico")
            .compile()
            .unwrap();
    }
}
