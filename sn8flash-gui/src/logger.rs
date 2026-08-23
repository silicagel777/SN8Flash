pub struct CallbackLogger {
    level: log::Level,
    callback: Box<dyn Fn(String) -> () + Send + Sync>,
}

impl CallbackLogger {
    pub fn new(level: log::Level, callback: Box<dyn Fn(String) -> () + Send + Sync>) -> Self {
        return Self { level, callback };
    }
}

impl log::Log for CallbackLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            (self.callback)(format!("{}: {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}
