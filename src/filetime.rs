use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct WindowsFileTime {
    value: i64,
}

impl WindowsFileTime {
    const UNIX_EPOCH_OFFSET_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    pub fn from_system_time(time: SystemTime) -> Self {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));
        let ticks = (duration.as_secs() + Self::UNIX_EPOCH_OFFSET_SECONDS) * Self::TICKS_PER_SECOND
            + u64::from(duration.subsec_nanos() / 100);
        Self {
            value: ticks as i64,
        }
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn to_file_time(value: i64) -> ::filetime::FileTime {
        let ticks_per_second = Self::TICKS_PER_SECOND as i64;
        let unix_offset = Self::UNIX_EPOCH_OFFSET_SECONDS as i64;
        let seconds_since_windows_epoch = value.div_euclid(ticks_per_second);
        let ticks_remainder = value.rem_euclid(ticks_per_second);
        ::filetime::FileTime::from_unix_time(
            seconds_since_windows_epoch - unix_offset,
            (ticks_remainder as u32) * 100,
        )
    }

    pub fn try_to_file_time(value: i64) -> Option<::filetime::FileTime> {
        (value > 0).then(|| Self::to_file_time(value))
    }
}
