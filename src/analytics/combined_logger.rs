pub struct CombinedLogger {
    log_tracer: tracing_log::LogTracer,
    env_logger: env_logger::Logger,
}

impl CombinedLogger {
    pub fn new() -> Self {
        let log_tracer = tracing_log::LogTracer::new();
        // default at info level
        let env_logger =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .target(env_logger::Target::Stdout)
                .build();

        Self {
            log_tracer,
            env_logger,
        }
    }
}

impl log::Log for CombinedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.env_logger.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.env_logger.log(record);
        self.log_tracer.log(record);
    }

    fn flush(&self) {
        self.env_logger.flush();
    }
}