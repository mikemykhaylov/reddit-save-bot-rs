use chrono::Utc;
use google_cloud_logging::{GCLogSeverity, GCOperation, GCSourceLocation, GoogleCloudStructLog};
use log::{Level, LevelFilter, Log};

pub struct Logger {
    operation_producer: String,
    error_report_type: String,
    max_log_level: LevelFilter,
}

impl Logger {
    pub fn new(max_log_level: Option<Level>, silent: Option<bool>) -> Logger {
        let max_log_level = match max_log_level {
            // if explicit level specified, use it
            Some(level) => level.to_level_filter(),
            // if silent mode enabled, use Error level
            // otherwise use Info level
            None => match silent {
                Some(true) => LevelFilter::Error,
                _ => LevelFilter::Trace,
            },
        };

        Logger {
            operation_producer: format!("{}:{}", "reddit-save-bot", env!("CARGO_PKG_VERSION")),
            error_report_type:
                "type.googleapis.com/google.devtools.clouderrorreporting.v1beta1.ReportedErrorEvent"
                    .to_string(),
            max_log_level,
        }
    }

    pub fn log_level_to_gc_severity(&self, level: Level) -> GCLogSeverity {
        match level {
            Level::Error => GCLogSeverity::Error,
            Level::Warn => GCLogSeverity::Warning,
            Level::Info => GCLogSeverity::Info,
            Level::Debug => GCLogSeverity::Debug,
            Level::Trace => GCLogSeverity::Debug,
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.max_log_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = self.log_level_to_gc_severity(record.level());

        // not sure if target is intended to be used this way
        let operation_id = record.metadata().target();

        let log_entry = GoogleCloudStructLog {
            severity: Some(level),
            message: Some(record.args().to_string()),
            report_type: match level {
                GCLogSeverity::Error => Some(self.error_report_type.clone()),
                _ => None,
            },
            time: Some(Utc::now()),
            operation: Some(GCOperation {
                id: Some(operation_id),
                producer: Some(&self.operation_producer),
                ..Default::default()
            }),
            source_location: Some(GCSourceLocation {
                file: Some(record.file().unwrap_or("")),
                line: Some(record.line().unwrap_or(0).to_string()),
                ..Default::default()
            }),
            trace: Some(operation_id.to_string()),
            ..Default::default()
        };
        println!(
            "{}",
            serde_json::to_string(&log_entry).expect("Failed to serialize log entry")
        );
    }

    fn flush(&self) {}
}

pub fn set_up_logger() {
    let logger = Logger::new(None, None);
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(LevelFilter::Info);
}
