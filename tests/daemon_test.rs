#![cfg(test)]
use std::env::temp_dir;

use core_lib::{core, logging::Logger};
use pretty_assertions::assert_eq;
use tokio::fs;

#[test]
fn test_id_generation() {
    let res = core::id::short_id();
    println!("generate id => {res}");
    assert_eq!(res.len(), 12);
}

#[tokio::test]
async fn test_log_basic() -> anyhow::Result<()> {
    let dir = temp_dir();
    let file_path = dir.join("test.log");
    let logger = Logger::new(&file_path).await?;

    logger.log("Hello World").await?;
    let contents = fs::read_to_string(&file_path).await?;
    assert!(contents.contains("Hello World"));

    Ok(())
}

#[tokio::test]
async fn test_info_waring_error() -> anyhow::Result<()> {
    let dir = temp_dir();
    let file_path = dir.join("multi.log");
    let logger = Logger::new(&file_path).await?;
    logger.info("test info").await?;
    logger.warning("test warning").await?;
    logger.error("test error").await?;

    let contents = fs::read_to_string(&file_path).await?;

    assert!(contents.contains("INFO: test info"));
    assert!(contents.contains("WARNING: test warning"));
    assert!(contents.contains("ERROR: test error"));
    Ok(())
}

#[tokio::test]
async fn test_concurent_log_writes() -> anyhow::Result<()> {
    let dir = temp_dir();
    let file_path = dir.join("multi.log");
    let logger = Logger::new(&file_path).await?;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let logger = logger.clone();
            tokio::spawn(async move {
                logger.info(&format!("message {i}")).await.unwrap();
            })
        })
        .collect();

    for handle in handles {
        handle.await?;
    }

    let contents = fs::read_to_string(&file_path).await?;
    for i in 0..10 {
        assert!(contents.contains(&format!("message {i}")));
    }

    Ok(())
}
