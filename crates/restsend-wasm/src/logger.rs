use log::{Level, Log, Metadata, Record};
use wasm_bindgen::prelude::*;
use web_sys::console;

struct WasmLogger {
    pub module_prefix: Option<String>,
}

impl Default for WasmLogger {
    fn default() -> Self {
        Self {
            module_prefix: None,
        }
    }
}

impl Log for WasmLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if let Some(ref prefix) = self.module_prefix {
            metadata.target().starts_with(prefix)
        } else {
            true
        }
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let file_line = match record.line() {
                Some(line) => format!(
                    "{}:{}",
                    record
                        .file()
                        .unwrap_or_else(|| record.target())
                        .split("/")
                        .last()
                        .unwrap_or_default(),
                    line
                ),
                None => "".to_string(),
            };
            let text = format!(
                "|RS|{}|{}|{}",
                record.level(),
                file_line,
                record.args().to_string()
            );
            let s = JsValue::from_str(&text);
            match record.level() {
                Level::Trace => console::debug_1(&s),
                Level::Debug => console::log_1(&s),
                Level::Info => console::info_1(&s),
                Level::Warn => console::warn_1(&s),
                Level::Error => console::error_1(&s),
            }
        }
    }

    fn flush(&self) {}
}

#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn setLogging(level: Option<String>) {
    let max_level = match level {
        Some(level) => match level.as_str() {
            "trace" => Level::Trace,
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            _ => Level::Warn,
        },
        _ => Level::Warn,
    };

    match log::set_boxed_logger(Box::new(WasmLogger::default())) {
        Ok(_) => log::set_max_level(max_level.to_level_filter()),
        Err(e) => console::error_1(&JsValue::from(e.to_string())),
    }
}
