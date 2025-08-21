use std::{collections::HashMap, env::temp_dir, sync::Arc};

use core_lib::{
    config::parser::ProjectConfig,
    core::{
        self,
        state::{AppState, init_watch_file, remove_watch_by_id},
        watcher::WatchContextBuilder,
    },
    git::repo::Repo,
    ipc::server::{DaemonResponse, handle_list_watches, handle_rm_watch, handle_up_watch},
    logging::Logger,
};
use pretty_assertions::assert_eq;
use tokio::{fs, sync::RwLock};

#[test]
fn test_id_generation() {
    let res = core::id::short_id();
    println!("generate id => {res}");
    println!("generate id => {res}");
    assert_eq!(res.len(), 12);
}

#[tokio::test]
async fn test_log_basic() -> anyhow::Result<()> {
    let dir = temp_dir();
    let file_path = dir.join("test.log");
    let logger = Logger::new(&file_path).await?;

    logger.info("Hello World").await?;
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

    assert!(contents.contains("test info"));
    assert!(contents.contains("test warning"));
    assert!(contents.contains("test error"));
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

// #[tokio::test]
// async fn test_handle_stop_watch_existing() -> anyhow::Result<()> {
//     init_watch_file().await?;
//     let id = "watch_stop".to_string();

//     let mut map = HashMap::new();
//     let ctx = WatchContext {
//         paused: false,
//         project_dir: "dir".to_string(),
//         branch: "main".to_string(),
//         repo: Repo {
//             branch: "main".to_string(),
//             last_commit: "abc".to_string(),
//             name: "name".to_string(),
//             remote: "".to_string(),
//         },
//         id: id.clone(),
//         config: ProjectConfig::default(),
//     };

//     map.insert(id.clone(), ctx);
//     let state = Arc::new(AppState {
//         watches: RwLock::new(map),
//     });

//     let response = handle_stop_watch(state.clone(), id.clone()).await;
//     remove_watch_by_id(&id).await?;

//     match response {
//         DaemonResponse::Success(msg) => {
//             assert!(msg.contains("Watch stopped"));
//         }
//         _ => panic!("Expected success response"),
//     }
//     Ok(())
// }

#[tokio::test]
async fn test_handle_up_watch_existing() -> anyhow::Result<()> {
    init_watch_file().await?;
    let id = "watch_up".to_string();

    let mut map = HashMap::new();

    let repo = Repo {
        branch: "main".to_string(),
        last_commit: "abc".to_string(),
        name: "name".to_string(),
        remote: "git://github.com/pepedinho/fleet.git".to_string(),
    };
    let ctx = WatchContextBuilder::new(
        "main".to_string(),
        repo,
        ProjectConfig::default(),
        "dir".to_string(),
        id.clone(),
    )
    .build()
    .await?;

    ctx.logger.clean().await?;
    map.insert(id.clone(), ctx);
    let state = Arc::new(AppState {
        watches: RwLock::new(map),
    });

    let response = handle_up_watch(state.clone(), id.clone()).await;
    remove_watch_by_id(&id).await?;

    match response {
        DaemonResponse::Success(msg) => {
            assert!(msg.contains("Watch up"));
        }
        _ => panic!("Excpected succes response"),
    }

    Ok(())
}

// #[tokio::test]
// async fn test_handle_rm_watch_existing() -> anyhow::Result<()> {
//     init_watch_file().await?;
//     let id = "rm_watch".to_string();

//     let mut map = HashMap::new();
//     let ctx = WatchContext {
//         paused: false,
//         project_dir: "dir".to_string(),
//         branch: "main".to_string(),
//         repo: Repo {
//             branch: "main".to_string(),
//             last_commit: "abc".to_string(),
//             name: "name".to_string(),
//             remote: "".to_string(),
//         },
//         id: id.clone(),
//         config: ProjectConfig::default(),
//     };
//     map.insert(id.clone(), ctx);

//     let state = Arc::new(AppState {
//         watches: RwLock::new(map),
//     });

//     let response = handle_rm_watch(state.clone(), id.clone()).await;

//     match response {
//         DaemonResponse::Success(msg) => assert!(msg.contains("was deleted")),
//         _ => panic!("Expected succes response"),
//     }
//     Ok(())
// }

#[tokio::test]
async fn test_handle_rm_non_existing() -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        watches: RwLock::new(HashMap::new()),
    });

    let response = handle_rm_watch(state.clone(), "unknown".to_string()).await;

    match response {
        DaemonResponse::Error(msg) => assert!(msg.contains("ID not found")),
        _ => panic!("Expected error response"),
    }
    Ok(())
}

#[tokio::test]
async fn test_handle_list_watches_existing() -> anyhow::Result<()> {
    let mut map = HashMap::new();
    let repo = Repo {
        branch: "main".to_string(),
        last_commit: "abc".to_string(),
        name: "name".to_string(),
        remote: "git://github.com/pepedinho/fleet.git".to_string(),
    };
    let ctx = WatchContextBuilder::new(
        "main".to_string(),
        repo,
        ProjectConfig::default(),
        "dir".to_string(),
        "watch1".to_string(),
    )
    .build()
    .await?;
    ctx.logger.clean().await?;

    map.insert("watch1".to_string(), ctx);

    let state = Arc::new(AppState {
        watches: RwLock::new(map),
    });

    let response = handle_list_watches(state.clone(), false).await;

    match response {
        DaemonResponse::ListWatches(list) => {
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].id, "watch1");
        }
        _ => panic!("Expected list watches"),
    }
    Ok(())
}

#[tokio::test]
async fn test_handle_list_watches_empty() -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        watches: RwLock::new(HashMap::new()),
    });

    let response = handle_list_watches(state.clone(), false).await;

    match response {
        DaemonResponse::ListWatches(list) => assert!(list.is_empty()),
        _ => panic!("Expected empty list"),
    }
    Ok(())
}
