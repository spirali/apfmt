fn f() {
    {
        let config_dir =
            if let Some(config_dir) = env::var_os("BAT_CONFIG_DIR").map(PathBuf::from) {
                config_dir
            } else {
                basedirs.config_dir().join("bat")
            };
        let max_counted_words =
            match std::env::var("MEILI_EXPERIMENTAL_DSR_FUEL_MAX_COUNTED_WORDS") {
                Ok(var) => var.parse()?,
                Err(std::env::VarError::NotPresent) => 10,
                Err(e) => return Err(e.into()),
            };
    }
    let naive_time =
        match chrono::NaiveTime::from_hms_micro_opt(hour, minute, second, time.micro()) {
            Some(v) => v,
            None => return Ok(0),
        };
}
