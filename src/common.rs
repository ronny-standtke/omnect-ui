macro_rules! config_path {
    ($filename:expr) => {{
        static CONFIG_PATH_DEFAULT: &'static str = "/data/config";
        Path::new(&std::env::var("CONFIG_PATH").unwrap_or(CONFIG_PATH_DEFAULT.to_string()))
            .join($filename)
    }};
}
pub(crate) use config_path;
