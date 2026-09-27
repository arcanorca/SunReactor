use std::path::Path;
use tz::TimeZone;

pub(crate) fn resolve_timezone(timezone_name: &str) -> Option<TimeZone> {
    let timezone_name = timezone_name.trim();

    from_system_zoneinfo(timezone_name)
        .or_else(|| from_embedded_iana(timezone_name))
        .or_else(|| TimeZone::from_posix_tz(timezone_name).ok())
}

fn from_system_zoneinfo(timezone_name: &str) -> Option<TimeZone> {
    let path = Path::new("/usr/share/zoneinfo").join(timezone_name);
    std::fs::read(path)
        .ok()
        .and_then(|data| TimeZone::from_tz_data(&data).ok())
}

fn from_embedded_iana(timezone_name: &str) -> Option<TimeZone> {
    tzdb::raw_tz_by_name(timezone_name).and_then(|data| TimeZone::from_tz_data(data).ok())
}

#[cfg(test)]
mod tests {
    use super::from_embedded_iana;

    #[test]
    fn embedded_database_resolves_iana_zone_without_os_zoneinfo() {
        let timezone = from_embedded_iana("Europe/Istanbul")
            .expect("embedded timezone database should contain Europe/Istanbul");
        let local_time = timezone
            .find_local_time_type(1_700_000_000)
            .expect("timezone should resolve the test timestamp");

        assert_eq!(local_time.ut_offset(), 10_800);
    }
}
