use warp::Filter;

use crate::http_server::routes::middlewares;
use crate::DependencyContainer;

pub fn routes(
    dependency_manager: &DependencyContainer,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    post_statistics(dependency_manager)
}

/// POST /statistics/snapshot
fn post_statistics(
    dependency_manager: &DependencyContainer,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("statistics" / "snapshot")
        .and(warp::post())
        .and(warp::body::json())
        .and(middlewares::with_event_transmitter(dependency_manager))
        .and_then(handlers::post_snapshot_statistics)
}

mod handlers {
    use std::{convert::Infallible, sync::Arc};

    use mithril_common::messages::SnapshotDownloadMessage;
    use warp::http::StatusCode;

    use crate::event_store::{EventMessage, TransmitterService};
    use crate::http_server::routes::reply;

    pub async fn post_snapshot_statistics(
        snapshot_download_message: SnapshotDownloadMessage,
        event_transmitter: Arc<TransmitterService<EventMessage>>,
    ) -> Result<impl warp::Reply, Infallible> {
        let headers: Vec<(&str, &str)> = Vec::new();

        match event_transmitter.send_event_message(
            "HTTP::statistics",
            "snapshot_downloaded",
            &snapshot_download_message,
            headers,
        ) {
            Err(e) => Ok(reply::internal_server_error(e)),
            Ok(_) => Ok(reply::empty(StatusCode::CREATED)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mithril_common::messages::SnapshotDownloadMessage;
    use mithril_common::test_utils::apispec::APISpec;

    use std::sync::Arc;
    use warp::{
        http::{Method, StatusCode},
        test::request,
    };

    use crate::{
        dependency_injection::DependenciesBuilder, http_server::SERVER_BASE_PATH, Configuration,
    };

    fn setup_router(
        dependency_manager: Arc<DependencyContainer>,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS]);

        warp::any()
            .and(warp::path(SERVER_BASE_PATH))
            .and(routes(&dependency_manager).with(cors))
    }

    #[tokio::test]
    async fn post_statistics_ok() {
        let config = Configuration::new_sample();
        let mut builder = DependenciesBuilder::new_with_stdout_logger(config);
        let mut rx = builder.get_event_transmitter_receiver().await.unwrap();
        let dependency_manager = builder.build_dependency_container().await.unwrap();
        let snapshot_download_message = SnapshotDownloadMessage::dummy();

        let method = Method::POST.as_str();
        let path = "/statistics/snapshot";

        let response = request()
            .method(method)
            .json(&snapshot_download_message)
            .path(&format!("/{SERVER_BASE_PATH}{path}"))
            .reply(&setup_router(Arc::new(dependency_manager)))
            .await;

        let result = APISpec::verify_conformity(
            APISpec::get_all_spec_files(),
            method,
            path,
            "application/json",
            &snapshot_download_message,
            &response,
            &StatusCode::CREATED,
        );

        let _ = rx.try_recv().unwrap();
        result.unwrap();
    }
}
