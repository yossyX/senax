use anyhow::{bail, Result};
use bytes::{Buf, BufMut, BytesMut};
use log::error;
use std::convert::TryInto;
use std::io::Write;
use std::path::Path;
use std::thread;
use time::{format_description, Duration, OffsetDateTime, Time, UtcOffset};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_uring::fs::{File, OpenOptions};
use zstd::Encoder;

// Based on tracing-appender

macro_rules! if_then_else {
    ( $if:expr, $then:expr, $else:expr ) => {
        if $if {
            $then
        } else {
            $else
        }
    };
}

#[derive(Clone, Debug)]
pub struct LogWriter {
    writer: UnboundedSender<String>,
}

impl LogWriter {
    pub fn new(
        rotation: Rotation,
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
        offset: UtcOffset,
        compress: bool,
    ) -> LogWriter {
        let log_directory = directory.as_ref().to_owned();
        let log_filename_prefix = file_name_prefix.as_ref().to_str().unwrap().to_string();
        let (writer, mut writer_rx) = mpsc::unbounded_channel::<String>();
        thread::Builder::new()
            .name("log writer".to_string())
            .spawn(move || {
                tokio_uring::start(async move {
                    let now = rotation.round_date(&OffsetDateTime::now_utc().to_offset(offset));
                    let (mut next_date, mut file) = Self::rotate(
                        &rotation,
                        &log_directory,
                        &log_filename_prefix,
                        &now,
                        compress,
                    )
                    .await
                    .unwrap();
                    while let Some(log) = writer_rx.recv().await {
                        let now = rotation.round_date(&OffsetDateTime::now_utc().to_offset(offset));
                        if let Some(next) = next_date {
                            if now.unix_timestamp() >= next.unix_timestamp() {
                                (next_date, file) = Self::rotate(
                                    &rotation,
                                    &log_directory,
                                    &log_filename_prefix,
                                    &now,
                                    compress,
                                )
                                .await
                                .unwrap();
                            }
                        }
                        if compress {
                            let mut sleep = std::time::Duration::from_millis(1000);
                            if let Some(next) = next_date {
                                sleep = std::cmp::min(sleep, (next - now).try_into().unwrap())
                            }
                            tokio::time::sleep(sleep).await;
                            let mut writer = BytesMut::with_capacity(262144).writer();
                            let mut enc = Encoder::new(&mut writer, 1).unwrap();
                            enc.write_all(log.as_bytes()).unwrap();
                            while let Ok(log) = writer_rx.try_recv() {
                                enc.write_all(log.as_bytes()).unwrap();
                            }
                            enc.finish().unwrap();
                            let _ = write(&file, writer.into_inner()).await;
                        } else {
                            let mut writer = BytesMut::with_capacity(log.len());
                            writer.put(log.as_bytes());
                            let _ = write(&file, writer).await;
                        }
                    }
                })
            })
            .unwrap();
        Self { writer }
    }

    async fn rotate(
        rotation: &Rotation,
        log_directory: &Path,
        log_filename_prefix: &str,
        now: &OffsetDateTime,
        compress: bool,
    ) -> Result<(Option<OffsetDateTime>, File)> {
        let filename = rotation.join_date(
            log_filename_prefix,
            now,
            if_then_else!(compress, Some("zst"), None),
        );
        let path = log_directory.join(filename);
        let next_date = rotation.next_date(now);
        let file = match OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(err) => {
                error!("{}", err);
                bail!(err);
            }
        };
        Ok((next_date, file))
    }

    pub fn minutely(
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
        offset: UtcOffset,
        compress: bool,
    ) -> LogWriter {
        LogWriter::new(
            Rotation::MINUTELY,
            directory,
            file_name_prefix,
            offset,
            compress,
        )
    }
    pub fn hourly(
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
        offset: UtcOffset,
        compress: bool,
    ) -> LogWriter {
        LogWriter::new(
            Rotation::HOURLY,
            directory,
            file_name_prefix,
            offset,
            compress,
        )
    }
    pub fn daily(
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
        offset: UtcOffset,
        compress: bool,
    ) -> LogWriter {
        LogWriter::new(
            Rotation::DAILY,
            directory,
            file_name_prefix,
            offset,
            compress,
        )
    }
    pub fn never(
        directory: impl AsRef<Path>,
        file_name: impl AsRef<Path>,
        compress: bool,
    ) -> LogWriter {
        LogWriter::new(
            Rotation::NEVER,
            directory,
            file_name,
            UtcOffset::UTC,
            compress,
        )
    }

    pub fn write(&self, log: String) {
        let _ = self.writer.send(log);
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rotation(RotationKind);

#[derive(Clone, PartialEq, Eq, Debug)]
enum RotationKind {
    Minutely,
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    /// Provides an minutely rotation
    pub const MINUTELY: Self = Self(RotationKind::Minutely);
    /// Provides an hourly rotation
    pub const HOURLY: Self = Self(RotationKind::Hourly);
    /// Provides a daily rotation
    pub const DAILY: Self = Self(RotationKind::Daily);
    /// Provides a rotation that never rotates.
    pub const NEVER: Self = Self(RotationKind::Never);

    pub(crate) fn next_date(&self, current_date: &OffsetDateTime) -> Option<OffsetDateTime> {
        let unrounded_next_date = match *self {
            Rotation::MINUTELY => *current_date + Duration::minutes(1),
            Rotation::HOURLY => *current_date + Duration::hours(1),
            Rotation::DAILY => *current_date + Duration::days(1),
            Rotation::NEVER => return None,
        };
        Some(self.round_date(&unrounded_next_date))
    }

    pub(crate) fn round_date(&self, date: &OffsetDateTime) -> OffsetDateTime {
        match *self {
            Rotation::MINUTELY => {
                let time = Time::from_hms(date.hour(), date.minute(), 0).unwrap();
                date.replace_time(time)
            }
            Rotation::HOURLY => {
                let time = Time::from_hms(date.hour(), 0, 0).unwrap();
                date.replace_time(time)
            }
            Rotation::DAILY => {
                let time = Time::from_hms(0, 0, 0).unwrap();
                date.replace_time(time)
            }
            Rotation::NEVER => *date,
        }
    }

    pub(crate) fn join_date(
        &self,
        filename: &str,
        date: &OffsetDateTime,
        suffix: Option<&str>,
    ) -> String {
        let format = match *self {
            Rotation::MINUTELY => format_description::parse("[year]-[month]-[day]-[hour]-[minute]"),
            Rotation::HOURLY => format_description::parse("[year]-[month]-[day]-[hour]"),
            Rotation::DAILY => format_description::parse("[year]-[month]-[day]"),
            Rotation::NEVER => format_description::parse(""),
        }
        .unwrap();
        let date = date.format(&format).unwrap();

        match (self, filename, suffix) {
            (&Rotation::NEVER, filename, None) => filename.to_string(),
            (&Rotation::NEVER, filename, Some(suffix)) => format!("{}.{}", filename, suffix),
            (_, filename, Some(suffix)) => format!("{}.{}.{}", filename, date, suffix),
            (_, filename, None) => format!("{}.{}", filename, date),
        }
    }
}

async fn write(file: &File, mut buf: BytesMut) -> Result<()> {
    loop {
        let (res, _buf) = file.write_at(buf, 0).await;
        buf = _buf;
        let len = res?;
        if len == 0 {
            bail!("write zero byte error");
        }
        if buf.len() > len {
            buf.advance(len);
            continue;
        }
        break;
    }
    Ok(())
}
