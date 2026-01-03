use gmticore::agp_interface::{DetectionRecord, ScenarioMetadata};
use iced::{
    mouse, time,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        column, row, scrollable, slider, text, text_input, Column, Container,
    },
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme,
};
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::PI,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn main() -> iced::Result {
    iced::application(Visualizer::boot, Visualizer::update, Visualizer::view)
        .title(application_title)
        .subscription(application_subscription)
        .theme(application_theme)
        .run()
}

fn application_title(_: &Visualizer) -> String {
    "GMTI-Rust Visualizer".into()
}

fn application_subscription(_: &Visualizer) -> Subscription<Message> {
    time::every(Duration::from_secs(1)).map(|_| Message::Tick)
}

fn application_theme(_: &Visualizer) -> Theme {
    Theme::Dark
}

const STREAM_DURATION_SECS: u32 = 600;

#[derive(Debug, Clone, Copy)]
struct StreamSession {
    remaining_secs: u32,
    elapsed_secs: u32,
    start_timestamp: f64,
}

#[derive(Debug)]
struct Visualizer {
    config: ConfigForm,
    payload: Option<VisualizationPayload>,
    waveform: Vec<f32>,
    status: String,
    history: Vec<String>,
    view_state: DetectionViewState,
    stream_session: Option<StreamSession>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    PayloadFetched(Result<VisualizationPayload, String>),
    ConfigFieldChanged(ConfigField, String),
    SubmitConfig,
    ConfigSubmitted(Result<String, String>),
    SetViewMode(DetectionViewMode),
    SetZoom(f32),
    SetRotation(f32),
    ToggleGrid,
    ToggleLabels,
    ResetView,
    StartRun,
    StopRun,
}

#[derive(Debug, Clone, Copy)]
enum ConfigField {
    Taps,
    RangeBins,
    DopplerBins,
    Frequency,
    Noise,
    Seed,
    Description,
    ScenarioName,
    PlatformType,
    PlatformVelocity,
    Altitude,
    AreaWidth,
    AreaHeight,
    ClutterLevel,
    SnrTarget,
    InterferenceLevel,
    TargetMotion,
}

impl Visualizer {
    fn boot() -> (Self, Task<Message>) {
        (
            Visualizer {
                config: ConfigForm::default(),
                payload: None,
                waveform: Vec::new(),
                status: "Waiting for telemetry...".into(),
                history: Vec::new(),
                view_state: DetectionViewState::default(),
                stream_session: None,
            },
            Task::perform(fetch_payload(), Message::PayloadFetched),
        )
    }

    fn update(state: &mut Self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let fetch_task = Task::perform(fetch_payload(), Message::PayloadFetched);
                if let Some(session) = state.stream_session.as_mut() {
                    if session.remaining_secs == 0 {
                        state.stream_session = None;
                        state.status = "Streaming run complete.".into();
                        return fetch_task;
                    }
                    let timestamp = session.start_timestamp + session.elapsed_secs as f64;
                    let config_payload = state.config.to_payload_with_timestamp(Some(timestamp));
                    session.elapsed_secs = session.elapsed_secs.saturating_add(1);
                    session.remaining_secs = session.remaining_secs.saturating_sub(1);
                    if session.remaining_secs == 0 {
                        state.stream_session = None;
                        state.status = "Streaming run complete.".into();
                    } else {
                        state.status =
                            format!("Streaming run: {}s remaining", session.remaining_secs);
                    }
                    let stream_task =
                        Task::perform(post_config(config_payload), Message::ConfigSubmitted);
                    return Task::batch(vec![fetch_task, stream_task]);
                }
                fetch_task
            }
            Message::PayloadFetched(Ok(payload)) => {
                state.waveform = payload.power_profile.clone();
                state.payload = Some(payload.clone());
                state.status = format!(
                    "Telemetry received: {} detections / {} bins",
                    payload.detection_count,
                    payload.power_profile.len()
                );
                state.push_history(format!(
                    "Telemetry: {} detections / {} bins",
                    payload.detection_count,
                    payload.power_profile.len()
                ));
                Task::none()
            }
            Message::PayloadFetched(Err(err)) => {
                state.status = format!("Telemetry error: {err}");
                Task::none()
            }
            Message::ConfigFieldChanged(field, value) => {
                state.config.update_field(field, value);
                Task::none()
            }
            Message::SubmitConfig => {
                let payload = state.config.to_payload();
                Task::perform(post_config(payload), Message::ConfigSubmitted)
            }
            Message::ConfigSubmitted(Ok(message)) => {
                if state.stream_session.is_none() {
                    state.status = message;
                }
                state.push_history("Scenario submitted".into());
                Task::none()
            }
            Message::ConfigSubmitted(Err(err)) => {
                state.status = format!("Config error: {err}");
                Task::none()
            }
            Message::SetViewMode(mode) => {
                state.view_state.mode = mode;
                Task::none()
            }
            Message::SetZoom(value) => {
                state.view_state.zoom = value;
                Task::none()
            }
            Message::SetRotation(value) => {
                state.view_state.rotation = value;
                Task::none()
            }
            Message::ToggleGrid => {
                state.view_state.show_grid = !state.view_state.show_grid;
                Task::none()
            }
            Message::ToggleLabels => {
                state.view_state.show_labels = !state.view_state.show_labels;
                Task::none()
            }
            Message::ResetView => {
                state.view_state = DetectionViewState::default();
                Task::none()
            }
            Message::StartRun => {
                let start_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|_| Duration::ZERO)
                    .as_secs_f64();
                state.stream_session = Some(StreamSession {
                    remaining_secs: STREAM_DURATION_SECS,
                    elapsed_secs: 0,
                    start_timestamp,
                });
                state.status = format!("Streaming run: {}s remaining", STREAM_DURATION_SECS);
                Task::none()
            }
            Message::StopRun => {
                state.stream_session = None;
                state.status = "Streaming run stopped".into();
                Task::none()
            }
        }
    }

    fn view(state: &Self) -> Element<'_, Message> {
        let detection_records = state
            .payload
            .as_ref()
            .map(|payload| payload.detection_records.clone())
            .unwrap_or_default();
        let detection_notes = state
            .payload
            .as_ref()
            .map(|payload| payload.detection_notes.clone())
            .unwrap_or_default();

        let config_column = column![
            text("Input Config").size(26),
            text_input("Taps", &state.config.taps)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Taps, value))
                .padding(6),
            text_input("Range bins", &state.config.range_bins)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::RangeBins, value))
                .padding(6),
            text_input("Doppler bins", &state.config.doppler_bins)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::DopplerBins, value))
                .padding(6),
            text_input("Frequency (Hz)", &state.config.frequency)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Frequency, value))
                .padding(6),
            text_input("Noise floor", &state.config.noise)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Noise, value))
                .padding(6),
            text_input("Seed", &state.config.seed)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Seed, value))
                .padding(6),
            text_input("Description", &state.config.description)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Description, value))
                .padding(6),
            text_input("Scenario name", &state.config.scenario_name)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::ScenarioName, value))
                .padding(6),
            text_input("Platform type", &state.config.platform_type)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::PlatformType, value))
                .padding(6),
            text_input("Platform velocity (km/h)", &state.config.platform_velocity)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::PlatformVelocity, value))
                .padding(6),
            text_input("Altitude (m)", &state.config.altitude)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::Altitude, value))
                .padding(6),
            text_input("Surveillance width (km)", &state.config.area_width)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::AreaWidth, value))
                .padding(6),
            text_input("Surveillance height (km)", &state.config.area_height)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::AreaHeight, value))
                .padding(6),
            text_input("Clutter level (0-1)", &state.config.clutter_level)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::ClutterLevel, value))
                .padding(6),
            text_input("Target SNR (dB)", &state.config.snr_target)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::SnrTarget, value))
                .padding(6),
            text_input("Interference (dB)", &state.config.interference_level)
                .on_input(|value| Message::ConfigFieldChanged(
                    ConfigField::InterferenceLevel,
                    value
                ))
                .padding(6),
            text_input("Target motion summary", &state.config.target_motion)
                .on_input(|value| Message::ConfigFieldChanged(ConfigField::TargetMotion, value))
                .padding(6),
            button("POST scenario")
                .on_press(Message::SubmitConfig)
                .padding(10),
            text(&state.status).size(14),
            column![
                text("Parameter definitions").size(16),
                text("Taps: pulses per CPI; increasing them deepens Doppler precision.").size(12),
                text("Range bins: samples per PRI; more bins map range more finely.").size(12),
                text("Doppler bins: FFT length referenced to the coherent processing interval.")
                    .size(12),
                text("Noise floor: simulated background clutter power (0-1).").size(12),
                text("Platform & motion: describe where the radar is mounted and how it moves.")
                    .size(12),
                text("Clutter level: ratio of unwanted echoes; higher values add terrain scatter.")
                    .size(12),
                text("Target SNR: expected signal-to-noise ratio for tracked emitters.").size(12),
                text("Interference: intentional or unintentional RF contamination in dB.").size(12),
                text(
                    "Target motion: narrative of how tactically relevant objects are maneuvering."
                )
                .size(12),
            ]
            .spacing(4)
            .padding(6),
        ]
        .spacing(10)
        .padding(16)
        .width(Length::Fixed(360.0));

        let detection_info = if let Some(payload) = &state.payload {
            text(format!(
                "Detections: {} / {} samples",
                payload.detection_count,
                payload.power_profile.len()
            ))
            .size(18)
        } else {
            text("Detections: n/a").size(18)
        };

        let stream_status = if let Some(session) = state.stream_session {
            format!("Streaming run: {}s remaining", session.remaining_secs)
        } else {
            "Streaming run idle".into()
        };
        let stream_button = if state.stream_session.is_some() {
            button("Stop 10-min run")
                .on_press(Message::StopRun)
                .padding(6)
        } else {
            button("Start 10-min run")
                .on_press(Message::StartRun)
                .padding(6)
        };
        let stream_controls = row![stream_button, text(stream_status).size(14)]
            .align_y(Alignment::Center)
            .spacing(10);

        let waveform = Canvas::new(Waveform {
            data: state.waveform.clone(),
        })
        .width(Length::Fill)
        .height(Length::Fixed(260.0));

        let scenario_metadata = state
            .payload
            .as_ref()
            .and_then(|payload| payload.scenario_metadata.clone());

        let detection_controls = column![
            text("Detection view controls").size(18),
            row![
                button("Polar view")
                    .on_press(Message::SetViewMode(DetectionViewMode::Polar))
                    .padding(4),
                button("Cartesian view")
                    .on_press(Message::SetViewMode(DetectionViewMode::Cartesian))
                    .padding(4),
            ]
            .spacing(10),
            row![
                column![
                    text("Zoom").size(12),
                    row![
                        slider(0.6..=2.5, state.view_state.zoom, Message::SetZoom)
                            .step(0.05)
                            .width(Length::FillPortion(2)),
                        text(format!("{:.2}x", state.view_state.zoom)).size(12)
                    ]
                    .spacing(4)
                ],
                column![
                    text("Rotation").size(12),
                    row![
                        slider(0.0..=360.0, state.view_state.rotation, Message::SetRotation)
                            .step(5.0)
                            .width(Length::FillPortion(2)),
                        text(format!("{:.0} deg", state.view_state.rotation)).size(12)
                    ]
                    .spacing(4)
                ]
            ]
            .spacing(10),
            row![
                button(if state.view_state.show_grid {
                    "Hide grid"
                } else {
                    "Show grid"
                })
                .on_press(Message::ToggleGrid)
                .padding(4),
                button(if state.view_state.show_labels {
                    "Hide labels"
                } else {
                    "Show labels"
                })
                .on_press(Message::ToggleLabels)
                .padding(4),
                button("Reset view").on_press(Message::ResetView).padding(4),
            ]
            .spacing(12),
        ]
        .spacing(6)
        .padding(6)
        .width(Length::Fill);

        let detection_canvas = Canvas::new(DetectionMap::new(
            &detection_records,
            state.view_state,
            scenario_metadata.clone(),
        ))
        .width(Length::Fill)
        .height(Length::Fixed(520.0));

        let tag_row = if let Some(metadata) = scenario_metadata.as_ref() {
            row![
                text(format!("Platform: {}", metadata.platform_type)).size(12),
                text(format!("Vel {:.0} km/h", metadata.platform_velocity_kmh)).size(12),
                text(format!(
                    "Area {:.1}×{:.1} km",
                    metadata.area_width_km, metadata.area_height_km
                ))
                .size(12),
                text(format!("Clutter {:.2}", metadata.clutter_level)).size(12),
                text(format!("SNR {:.1} dB", metadata.snr_target_db)).size(12),
            ]
            .spacing(12)
        } else {
            row![text("Metadata tags pending...").size(12)]
        };

        let axis_hint = if state.view_state.show_labels {
            if let Some(metadata) = scenario_metadata.as_ref() {
                text(format!(
                    "Area {:.1} km × {:.1} km | Clutter {:.2} | SNR target {:.1} dB",
                    metadata.area_width_km,
                    metadata.area_height_km,
                    metadata.clutter_level,
                    metadata.snr_target_db
                ))
                .size(12)
            } else {
                text("Axis metadata pending...").size(12)
            }
        } else {
            text("Axis labels hidden").size(12)
        };

        let detection_entries = if detection_records.is_empty() {
            Column::new().push(text("No detections to render").size(12))
        } else {
            detection_records.iter().enumerate().take(6).fold(
                Column::new().spacing(4),
                |col, (idx, detection)| {
                    col.push(
                        text(format!(
                            "#{}: range {:.1} m | doppler {:.2} m/s | SNR {:.2} dB",
                            idx + 1,
                            detection.range,
                            detection.doppler,
                            detection.snr
                        ))
                        .size(12),
                    )
                },
            )
        };

        let metadata_panel = if let Some(metadata) = scenario_metadata.clone() {
            column![
                text("Scenario metadata").size(16),
                text(format!("Scenario: {}", metadata.name)).size(12),
                text(format!("Platform: {}", metadata.platform_type)).size(12),
                text(format!(
                    "Velocity: {:.0} km/h",
                    metadata.platform_velocity_kmh
                ))
                .size(12),
                text(format!(
                    "Altitude: {} m",
                    metadata
                        .altitude_m
                        .map(|alt| alt.to_string())
                        .unwrap_or_else(|| "n/a".into())
                ))
                .size(12),
                text(format!(
                    "Area: {:.1} km × {:.1} km",
                    metadata.area_width_km, metadata.area_height_km
                ))
                .size(12),
                text(format!("Clutter level: {:.2}", metadata.clutter_level)).size(12),
                text(format!("Target SNR: {:.1} dB", metadata.snr_target_db)).size(12),
                text(format!("Interference: {:.1} dB", metadata.interference_db)).size(12),
                text(format!("Target motion: {}", metadata.target_motion)).size(12),
                metadata
                    .description
                    .as_ref()
                    .map(|description| text(format!("Notes: {}", description)).size(12))
                    .unwrap_or_else(|| text("Notes: n/a").size(12)),
            ]
            .spacing(4)
            .padding(8)
        } else {
            column![
                text("Scenario metadata").size(16),
                text("No metadata has arrived yet.").size(12),
            ]
            .spacing(4)
            .padding(8)
        };

        let notes_list = if detection_notes.is_empty() {
            Column::new().push(text("No notes yet").size(14))
        } else {
            detection_notes
                .iter()
                .rev()
                .fold(Column::new().spacing(4), |col, note| {
                    col.push(text(note.clone()).size(14))
                })
        };

        let history_list = if state.history.is_empty() {
            Column::new().push(text("No activity yet").size(12))
        } else {
            state
                .history
                .iter()
                .rev()
                .fold(Column::new().spacing(4), |col, entry| {
                    col.push(text(entry.clone()).size(12))
                })
        };

        let telemetry_column = column![
            text("Telemetry").size(26),
            detection_info,
            stream_controls,
            text("Power profile").size(18),
            waveform,
            detection_controls,
            text("Detection environment").size(18),
            detection_canvas,
            tag_row,
            axis_hint,
            metadata_panel,
            text("Recent detections").size(16),
            Container::new(detection_entries).padding(6),
            text("Processing notes").size(16),
            Container::new(scrollable(notes_list).height(Length::Fixed(120.0))).padding(6),
            text("Activity log").size(16),
            Container::new(scrollable(history_list).height(Length::Fixed(90.0))).padding(6),
        ]
        .spacing(10)
        .padding(16)
        .width(Length::Fill);

        let layout = row![config_column, telemetry_column]
            .spacing(20)
            .align_y(Alignment::Start)
            .padding(20);

        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn push_history(&mut self, entry: String) {
        self.history.push(entry);
        if self.history.len() > 20 {
            self.history.remove(0);
        }
    }
}

async fn fetch_payload() -> Result<VisualizationPayload, String> {
    let response = reqwest::get("http://127.0.0.1:9000/payload")
        .await
        .map_err(|e| e.to_string())?;
    response
        .json::<VisualizationPayload>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_config(config: ScenarioConfig) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:9000/ingest-config")
        .json(&config)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok("Scenario submitted".into())
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "".into());
        Err(format!("{}: {}", status, text))
    }
}

#[derive(Debug, Clone)]
struct ConfigForm {
    taps: String,
    range_bins: String,
    doppler_bins: String,
    frequency: String,
    noise: String,
    seed: String,
    description: String,
    scenario_name: String,
    platform_type: String,
    platform_velocity: String,
    altitude: String,
    area_width: String,
    area_height: String,
    clutter_level: String,
    snr_target: String,
    interference_level: String,
    target_motion: String,
}

impl ConfigForm {
    fn default() -> Self {
        Self {
            taps: "4".into(),
            range_bins: "2048".into(),
            doppler_bins: "256".into(),
            frequency: "1050000000".into(),
            noise: "0.07".into(),
            seed: "312".into(),
            description: "Rust visualizer scenario".into(),
            scenario_name: "Airborne sweep".into(),
            platform_type: "Airborne ISR".into(),
            platform_velocity: "750".into(),
            altitude: "8200".into(),
            area_width: "10".into(),
            area_height: "10".into(),
            clutter_level: "0.45".into(),
            snr_target: "18".into(),
            interference_level: "-10".into(),
            target_motion: "Cruise, gentle zig-zag".into(),
        }
    }

    fn update_field(&mut self, field: ConfigField, value: String) {
        match field {
            ConfigField::Taps => self.taps = value,
            ConfigField::RangeBins => self.range_bins = value,
            ConfigField::DopplerBins => self.doppler_bins = value,
            ConfigField::Frequency => self.frequency = value,
            ConfigField::Noise => self.noise = value,
            ConfigField::Seed => self.seed = value,
            ConfigField::Description => self.description = value,
            ConfigField::ScenarioName => self.scenario_name = value,
            ConfigField::PlatformType => self.platform_type = value,
            ConfigField::PlatformVelocity => self.platform_velocity = value,
            ConfigField::Altitude => self.altitude = value,
            ConfigField::AreaWidth => self.area_width = value,
            ConfigField::AreaHeight => self.area_height = value,
            ConfigField::ClutterLevel => self.clutter_level = value,
            ConfigField::SnrTarget => self.snr_target = value,
            ConfigField::InterferenceLevel => self.interference_level = value,
            ConfigField::TargetMotion => self.target_motion = value,
        }
    }

    fn to_payload(&self) -> ScenarioConfig {
        ScenarioConfig {
            taps: self.taps.parse().ok(),
            range_bins: self.range_bins.parse().ok(),
            doppler_bins: self.doppler_bins.parse().ok(),
            frequency: self.frequency.parse().ok(),
            noise: self.noise.parse().ok(),
            seed: self.seed.parse().ok(),
            description: if self.description.trim().is_empty() {
                None
            } else {
                Some(self.description.clone())
            },
            scenario: if self.scenario_name.trim().is_empty() {
                None
            } else {
                Some(self.scenario_name.clone())
            },
            platform_type: if self.platform_type.trim().is_empty() {
                None
            } else {
                Some(self.platform_type.clone())
            },
            platform_velocity_kmh: self.platform_velocity.parse().ok(),
            altitude_m: self.altitude.parse().ok(),
            area_width_km: self.area_width.parse().ok(),
            area_height_km: self.area_height.parse().ok(),
            clutter_level: self.clutter_level.parse().ok(),
            snr_target_db: self.snr_target.parse().ok(),
            interference_db: self.interference_level.parse().ok(),
            target_motion: if self.target_motion.trim().is_empty() {
                None
            } else {
                Some(self.target_motion.clone())
            },
            timestamp_start: None,
        }
    }

    fn to_payload_with_timestamp(&self, timestamp: Option<f64>) -> ScenarioConfig {
        let mut payload = self.to_payload();
        payload.timestamp_start = timestamp;
        payload
    }
}

#[derive(Debug, Serialize)]
struct ScenarioConfig {
    taps: Option<u32>,
    range_bins: Option<u32>,
    doppler_bins: Option<u32>,
    frequency: Option<f32>,
    noise: Option<f32>,
    seed: Option<u64>,
    description: Option<String>,
    scenario: Option<String>,
    platform_type: Option<String>,
    platform_velocity_kmh: Option<f32>,
    altitude_m: Option<f32>,
    area_width_km: Option<f32>,
    area_height_km: Option<f32>,
    clutter_level: Option<f32>,
    snr_target_db: Option<f32>,
    interference_db: Option<f32>,
    target_motion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp_start: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct VisualizationPayload {
    #[serde(default)]
    power_profile: Vec<f32>,
    #[serde(default)]
    detection_count: usize,
    #[serde(default)]
    detection_notes: Vec<String>,
    #[serde(default)]
    detection_records: Vec<DetectionRecord>,
    #[serde(default)]
    scenario_metadata: Option<ScenarioMetadata>,
}

#[derive(Clone)]
struct Waveform {
    data: Vec<f32>,
}

impl canvas::Program<Message> for Waveform {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.05, 0.05, 0.05),
        );

        if self.data.len() > 1 {
            let min = self.data.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = self.data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let range = (max - min).max(1.0);
            let width = bounds.width;
            let step = width / (self.data.len() as f32 - 1.0);
            let path = Path::new(|builder| {
                for (i, value) in self.data.iter().enumerate() {
                    let x = i as f32 * step;
                    let normalized = (value - min) / range;
                    let y = bounds.height - normalized * bounds.height;
                    if i == 0 {
                        builder.move_to(Point::new(x, y));
                    } else {
                        builder.line_to(Point::new(x, y));
                    }
                }
            });

            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(2.5)
                    .with_color(Color::from_rgb(0.18, 0.72, 0.89)),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectionViewMode {
    Polar,
    Cartesian,
}

#[derive(Debug, Clone, Copy)]
struct DetectionViewState {
    mode: DetectionViewMode,
    zoom: f32,
    rotation: f32,
    show_grid: bool,
    show_labels: bool,
}

impl Default for DetectionViewState {
    fn default() -> Self {
        Self {
            mode: DetectionViewMode::Polar,
            zoom: 1.0,
            rotation: 0.0,
            show_grid: true,
            show_labels: true,
        }
    }
}

#[derive(Clone)]
struct DetectionMap {
    records: Vec<DetectionRecord>,
    view: DetectionViewState,
    metadata: Option<ScenarioMetadata>,
}

impl DetectionMap {
    fn new(
        records: &[DetectionRecord],
        view: DetectionViewState,
        metadata: Option<ScenarioMetadata>,
    ) -> Self {
        Self {
            records: records.to_vec(),
            view,
            metadata,
        }
    }
}

impl canvas::Program<Message> for DetectionMap {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.01, 0.01, 0.03),
        );

        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let base_radius = bounds.width.min(bounds.height) / 2.0 - 12.0;
        let zoom = self.view.zoom.clamp(0.6, 2.5);
        let radius = (base_radius * zoom).max(16.0);

        if self.view.show_grid {
            match self.view.mode {
                DetectionViewMode::Polar => {
                    for ring in 1..=4 {
                        let ring_radius = radius * (ring as f32 / 4.0);
                        let ring_path = Path::new(|builder| builder.circle(center, ring_radius));
                        frame.stroke(
                            &ring_path,
                            Stroke::default().with_color(Color::from_rgb(0.25, 0.25, 0.32)),
                        );
                    }
                    let axes = Path::new(|builder| {
                        builder.move_to(Point::new(center.x - radius, center.y));
                        builder.line_to(Point::new(center.x + radius, center.y));
                        builder.move_to(Point::new(center.x, center.y - radius));
                        builder.line_to(Point::new(center.x, center.y + radius));
                    });
                    frame.stroke(
                        &axes,
                        Stroke::default()
                            .with_color(Color::from_rgb(0.35, 0.35, 0.45))
                            .with_width(1.0),
                    );
                }
                DetectionViewMode::Cartesian => {
                    let grid = Path::new(|builder| {
                        for lane in 0..=4 {
                            let offset = lane as f32 / 4.0;
                            let x = center.x + (offset * 2.0 - 1.0) * radius;
                            builder.move_to(Point::new(x, center.y - radius));
                            builder.line_to(Point::new(x, center.y + radius));
                            let y = center.y + (offset * 2.0 - 1.0) * radius;
                            builder.move_to(Point::new(center.x - radius, y));
                            builder.line_to(Point::new(center.x + radius, y));
                        }
                    });
                    frame.stroke(
                        &grid,
                        Stroke::default()
                            .with_color(Color::from_rgb(0.18, 0.25, 0.32))
                            .with_width(1.0),
                    );
                    let axes = Path::new(|builder| {
                        builder.move_to(Point::new(center.x - radius, center.y));
                        builder.line_to(Point::new(center.x + radius, center.y));
                        builder.move_to(Point::new(center.x, center.y - radius));
                        builder.line_to(Point::new(center.x, center.y + radius));
                    });
                    frame.stroke(
                        &axes,
                        Stroke::default()
                            .with_color(Color::from_rgb(0.40, 0.40, 0.45))
                            .with_width(1.2),
                    );
                }
            }
        }

        let metadata_range = self
            .metadata
            .as_ref()
            .map(|meta| meta.area_width_km.max(meta.area_height_km) * 1000.0)
            .unwrap_or(0.0);
        let max_range = self
            .records
            .iter()
            .map(|record| record.range)
            .fold(0.0, f32::max)
            .max(1.0);
        let display_range = metadata_range.max(max_range).max(1.0);
        let max_doppler = self
            .records
            .iter()
            .map(|record| record.doppler.abs())
            .fold(0.0, f32::max)
            .max(0.5);
        let rotation_rad = self.view.rotation.to_radians();

        for record in &self.records {
            let normalized_range = (record.range / display_range).clamp(0.0, 1.0);
            let normalized_doppler = if max_doppler > 0.0 {
                (record.doppler / max_doppler).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let (x, y) = match self.view.mode {
                DetectionViewMode::Polar => {
                    let point_radius = normalized_range * radius;
                    let angle = normalized_doppler * PI;
                    (
                        center.x + point_radius * angle.cos(),
                        center.y - point_radius * angle.sin(),
                    )
                }
                DetectionViewMode::Cartesian => (
                    center.x + (normalized_range * 2.0 - 1.0) * radius,
                    center.y - normalized_doppler * radius,
                ),
            };
            let rotated = rotate_point(Point::new(x, y), center, rotation_rad);
            let marker_radius = 3.0 + (record.snr.min(12.0) * 0.2);
            let marker = Path::new(|builder| builder.circle(rotated, marker_radius));
            let color = Color::from_rgb(
                0.25 + (record.snr / 40.0).clamp(0.0, 0.5),
                0.5 - (record.snr / 70.0).clamp(0.0, 0.3),
                0.2,
            );
            frame.fill(&marker, color);
        }

        vec![frame.into_geometry()]
    }
}

fn rotate_point(point: Point, center: Point, angle_rad: f32) -> Point {
    let sin = angle_rad.sin();
    let cos = angle_rad.cos();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}
