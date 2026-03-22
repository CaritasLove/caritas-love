// logging.rs
// Copyright 2026 Patrick Meade.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::Path;

use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, Logger, LoggerHandle, Naming,
    WriteMode,
};
use log::Record;

pub fn init_logging<P: AsRef<Path>>(
    log_dir: P,
) -> Result<LoggerHandle, Box<dyn std::error::Error>> {
    let log_dir = log_dir.as_ref();

    let handle = Logger::try_with_env_or_str("info")?
        .log_to_file(
            FileSpec::default()
                .directory(log_dir)
                .basename("caritas-love")
                .suffix("log"),
        )
        .format_for_files(log_format)
        .duplicate_to_stderr(Duplicate::Warn)
        .rotate(
            Criterion::Age(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepCompressedFiles(30),
        )
        .write_mode(WriteMode::Async)
        .print_message()
        .start()?;

    Ok(handle)
}

fn log_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{} [{}] {:<5} ({}:{}) - {}",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        std::thread::current().name().unwrap_or("main"),
        record.level(),
        record.file().unwrap_or("<unknown>"),
        record.line().unwrap_or(0),
        record.args()
    )
}
