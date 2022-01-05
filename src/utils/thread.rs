use std::io::{Error, ErrorKind, Result};
use tokio::fs::read_to_string;

pub async fn num_thread() -> Result<u64> {
    count().await
}

#[cfg(target_os = "linux")]
async fn count() -> Result<u64> {
    let stat = read_to_string("/proc/self/stat").await?;
    stat.split(" ")
        .nth(19)
        .ok_or(Error::new(
            ErrorKind::InvalidData,
            "No 20th element in /proc/stat",
        ))
        .and_then(|count| {
            count
                .parse::<u64>()
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Not a number"))
        })
}

#[cfg(not(target_os = "linux"))]
async fn count() -> Result<u64> {
    Ok(1)
}
