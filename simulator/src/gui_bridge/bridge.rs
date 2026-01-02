use crate::generator::profile::{build_pri_payload_from_config, GeneratorConfig};
use crate::gui_bridge::model::VisualizationModel;
use crate::workflow::runner::Runner;
use anyhow::Result;
use gmticore::agp_interface::PriPayload;
use serde_json::json;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
};
use tokio::runtime::Builder;
use warp::{http::StatusCode, Filter};

fn gui_bind_address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 9000))
}

#[derive(Debug)]
struct WarpError;

impl warp::reject::Reject for WarpError {}

/// Bridge that hosts the telemetry HTTP endpoint and processes incoming payloads.
pub struct GuiBridge {
    state: Arc<RwLock<VisualizationModel>>,
}

impl GuiBridge {
    pub fn new(runner: Arc<Runner>) -> Self {
        let state = Arc::new(RwLock::new(VisualizationModel::default()));
        let state_for_filter = state.clone();
        let state_filter = warp::any().map(move || state_for_filter.clone());
        let runner_filter = warp::any().map(move || runner.clone());

        let get_route = warp::path("payload")
            .and(warp::get())
            .and(state_filter.clone())
            .map(|state: Arc<RwLock<VisualizationModel>>| {
                warp::reply::json(&*state.read().unwrap())
            });

        let post_route = warp::path("ingest")
            .and(warp::post())
            .and(warp::body::json())
            .and(state_filter.clone())
            .and(runner_filter.clone())
            .and_then(
                |payload: PriPayload,
                 state: Arc<RwLock<VisualizationModel>>,
                 runner: Arc<Runner>| async move {
                    match runner.execute(&payload) {
                        Ok(result) => {
                            let mut guard = state.write().unwrap();
                            *guard = VisualizationModel {
                                power_profile: result.power_profile.clone(),
                                detection_count: result.detection_count,
                            };
                            Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(&json!({"status": "ok"})),
                                StatusCode::OK,
                            ))
                        }
                        Err(err) => {
                            eprintln!("ingest error: {}", err);
                            Err(warp::reject::custom(WarpError))
                        }
                    }
                },
            );

        let generator_route = warp::path("ingest-config")
            .and(warp::post())
            .and(warp::body::json())
            .and(state_filter)
            .and(runner_filter)
            .and_then(
                |config: GeneratorConfig,
                 state: Arc<RwLock<VisualizationModel>>,
                 runner: Arc<Runner>| async move {
                    match build_pri_payload_from_config(&config)
                        .and_then(|payload| runner.execute(&payload))
                    {
                        Ok(result) => {
                            let mut guard = state.write().unwrap();
                            *guard = VisualizationModel {
                                power_profile: result.power_profile.clone(),
                                detection_count: result.detection_count,
                            };
                            if let Some(name) = config.scenario.as_ref() {
                                println!(
                                    "[GUI] Scenario {} -> detections {}",
                                    name, result.detection_count
                                );
                            }
                            Ok::<_, warp::Rejection>(warp::reply::with_status(
                                warp::reply::json(&json!({
                                    "status": "ok",
                                    "detections": result.detection_count,
                                    "description": config.description.clone().unwrap_or_default()
                                })),
                                StatusCode::OK,
                            ))
                        }
                        Err(err) => {
                            eprintln!("ingest-config error: {}", err);
                            Err(warp::reject::custom(WarpError))
                        }
                    }
                },
            );

        thread::spawn(move || {
            let routes = get_route.or(post_route).or(generator_route);
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");
            runtime.block_on(async move {
                warp::serve(routes).run(gui_bind_address()).await;
            });
        });

        Self { state }
    }

    pub fn publish(&self, model: &VisualizationModel) -> Result<()> {
        let mut guard = self.state.write().unwrap();
        *guard = model.clone();
        println!(
            "[GUI] power profile points: {}, detections: {}",
            guard.power_profile.len(),
            guard.detection_count
        );
        Ok(())
    }

    pub fn publish_status(&self, message: &str) {
        println!("[GUI] {}", message);
    }

    #[cfg(test)]
    pub fn snapshot(&self) -> VisualizationModel {
        self.state.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::profile::build_pri_payload;
    use crate::workflow::config::WorkflowConfig;
    use crate::workflow::runner::Runner;
    use std::sync::Arc;

    #[test]
    fn gui_bridge_updates_state() {
        let cfg = WorkflowConfig::from_args(1, 8, 4);
        let runner = Arc::new(Runner::new(cfg.clone()));
        let gui = GuiBridge::new(runner.clone());
        let payload = build_pri_payload(cfg.taps, cfg.range_bins).unwrap();
        let result = runner.execute(&payload).unwrap();
        let model = VisualizationModel {
            power_profile: result.power_profile.clone(),
            detection_count: result.detection_count,
        };
        gui.publish(&model).unwrap();
        assert_eq!(gui.snapshot().detection_count, result.detection_count);
    }
}
