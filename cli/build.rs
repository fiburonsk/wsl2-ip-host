fn main() {
    #[cfg(windows)]
    win::run();
}

mod win {
    use winres::WindowsResource;

    pub fn run() {
        WindowsResource::new()
            .set_icon("./../resources/icon.ico")
            .compile()
            .unwrap();
    }
}
