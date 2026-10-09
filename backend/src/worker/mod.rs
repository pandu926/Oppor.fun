pub mod indexer;
pub mod jobs;
use crate::state::AppState;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let (worker_stop, stop_rx) = tokio::sync::watch::channel(false);
    let index_state = state.clone();
    let mut index_stop = stop_rx.clone();
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move {
        loop {
            if *index_stop.borrow() {
                break;
            }
            if let Err(e) = indexer::tick(&index_state).await {
                tracing::error!(error=%e,"Chain indexing failed");
            }
            tokio::select! {_=tokio::time::sleep(std::time::Duration::from_secs(2))=>{},_=index_stop.changed()=>{}}
        }
    });
    let job_state = state.clone();
    let mut job_stop = stop_rx.clone();
    tasks.spawn(async move {
        let worker = uuid::Uuid::new_v4();
        loop {
            if *job_stop.borrow() {
                break;
            }
            if let Err(e) = jobs::tick(&job_state, worker).await {
                tracing::error!(error=%e,"Allocation job failed");
            }
            tokio::select! {_=tokio::time::sleep(std::time::Duration::from_millis(500))=>{},_=job_stop.changed()=>{}}
        }
    });
    let maintenance_state = state.clone();
    let mut maintenance_stop = stop_rx;
    tasks.spawn(async move {
        loop {
            if *maintenance_stop.borrow() {
                break;
            }
            if let Err(e) = jobs::maintenance(&maintenance_state).await {
                tracing::error!(error=%e,"Maintenance failed");
            }
            tokio::select! {_=tokio::time::sleep(std::time::Duration::from_secs(60))=>{},_=maintenance_stop.changed()=>{}}
        }
    });
    let failed = tokio::select! {
        _ = shutdown() => false,
        task = tasks.join_next() => {
            tracing::error!(result=?task,"Worker task exited unexpectedly");
            true
        }
    };
    let _ = worker_stop.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while let Some(result) = tasks.join_next().await {
            result?;
        }
        Ok::<_, tokio::task::JoinError>(())
    })
    .await??;
    state.db.close().await;
    anyhow::ensure!(!failed, "A worker task exited unexpectedly");
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Install SIGTERM handler");
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("Install shutdown handler");
}
