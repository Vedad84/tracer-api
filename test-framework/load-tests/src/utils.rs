use std::time::Duration;

pub fn median(times: &[Duration]) -> Duration {
    assert!(times.len() > 0);

    let middle = times.len() / 2;
    if times.len() % 2 == 1 {
        times[middle]
    } else {
        (times[middle - 1] + times[middle]) / 2
    }
}
