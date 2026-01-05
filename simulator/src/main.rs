use anyhow::Context;
use clap::Parser;
use generator::profile::build_pri_payload;
use gui_bridge::bridge::GuiBridge;
use gui_bridge::model::VisualizationModel;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder as TokioBuilder;
use tokio::signal;
use workflow::config::WorkflowConfig;
use workflow::runner::Runner;

mod generator;
mod gui_bridge;
mod workflow;

#[derive(Parser)]
#[command(author, version, about = "Rust-facing GMTI workflow driver")]
struct Args {
    /// Run a single offline CPI wave and emit a baseline summary
    #[arg(long, default_value_t = false)]
    offline: bool,
    /// Load a workflow config from YAML
    #[arg(long)]
    workflow: Option<PathBuf>,
    #[arg(long, default_value_t = 4)]
    taps: usize,
    #[arg(long, default_value_t = 1024)]
    range_bins: usize,
    #[arg(long, default_value_t = 128)]
    doppler_bins: usize,
    /// Keep the GUI bridge alive for incoming real-time payloads
    #[arg(long, default_value_t = false)]
    serve: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let workflow_config = if let Some(path) = args.workflow {
        WorkflowConfig::load(path)?
    } else {
        WorkflowConfig::from_args(args.taps, args.range_bins, args.doppler_bins)
    };

    let runner = Runner::new(workflow_config.clone());
    let gui_bridge = GuiBridge::new(Arc::new(runner.clone()));
    let payload = build_pri_payload(workflow_config.taps, workflow_config.range_bins)?;

    if args.offline {
        let result = runner.execute(&payload)?;

        println!(
            "Offline run -> detections {}, power_profile len {}, records {}",
            result.detection_count,
            result.power_profile.len(),
            result.detection_records.len()
        );

        let model = VisualizationModel {
            power_profile: result.power_profile.clone(),
            detection_count: result.detection_count,
            detection_records: result.detection_records.clone(),
            detection_notes: result.doppler_notes.clone(),
            scenario_metadata: result.scenario_metadata.clone(),
        };

        gui_bridge.publish(&model)?;
        gui_bridge.publish_status("Offline workflow results ready.");

        let report = format!(
            "detections={} range_profile={} records={} doppler_notes={:?}\n",
            result.detection_count,
            result.power_profile.len(),
            result.detection_records.len(),
            result.doppler_notes
        );
        let report_path = PathBuf::from("tools/data/offline_detection.log");
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(report_path)?;
        file.write_all(report.as_bytes())?;
    }
    if args.serve {
        gui_bridge.publish_status("HTTP bridge running (Ctrl+C to stop)...");
        let runtime = TokioBuilder::new_current_thread()
            .enable_all()
            .build()
            .context("creating runtime for signal handling")?;
        runtime.block_on(async {
            signal::ctrl_c().await.context("awaiting Ctrl+C to exit")?;
            Ok::<(), anyhow::Error>(())
        })?;
    }

    Ok(())
}
