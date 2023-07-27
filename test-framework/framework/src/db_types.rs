use clickhouse::Row;
use serde::Deserialize;
use time::OffsetDateTime;

// This wrapper is only needed for a convenient reading of the time values from the database.
#[derive(Debug, Deserialize, Row)]
pub struct RetrievedTime {
    #[serde(with = "clickhouse::serde::time::datetime64::millis")]
    time: OffsetDateTime,
}

impl From<RetrievedTime> for OffsetDateTime {
    fn from(value: RetrievedTime) -> Self {
        value.time
    }
}
