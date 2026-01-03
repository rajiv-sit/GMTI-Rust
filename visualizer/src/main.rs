use gmticore::agp_interface::DetectionRecord;
use iced::{
    mouse, time,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry, Path, Stroke},
        column, row, scrollable, text, text_input, Column, Container,
    },
    Alignment, Color, Element, Length, Point, Rectangle, Renderer, Subscription, Task, Theme,
};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, time::Duration};

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

#[derive(Debug)]
struct Visualizer {
    config: ConfigForm,
    payload: Option<VisualizationPayload>,
    waveform: Vec<f32>,
    status: String,
    history: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    PayloadFetched(Result<VisualizationPayload, String>),
    ConfigFieldChanged(ConfigField, String),
    SubmitConfig,
    ConfigSubmitted(Result<String, String>),
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
            },
            Task::perform(fetch_payload(), Message::PayloadFetched),
        )
    }

    fn update(state: &mut Self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => Task::perform(fetch_payload(), Message::PayloadFetched),
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
                state.status = message;
                state.push_history("Scenario submitted".into());
                Task::none()
            }
            Message::ConfigSubmitted(Err(err)) => {
                state.status = format!("Config error: {err}");
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
            button("POST scenario")
                .on_press(Message::SubmitConfig)
                .padding(10),
            text(&state.status).size(14),
            column![
                text("Parameter definitions").size(16),
                text("Taps: pulses per coherent processing interval; more pulses improve Doppler resolution.")
                    .size(12),
                text("Range bins: number of samples per PRI; affects range detail.")
                    .size(12),
                text("Doppler bins: FFT length applied to each CPI; controls velocity resolution.")
                    .size(12),
                text("Frequency: radar carrier frequency (Hz) used to contextualize waveforms.")
                    .size(12),
                text("Noise floor: generator noise amplitude, simulating environmental clutter.")
                    .size(12),
                text("Seed: deterministic PRNG seeding so scenarios replay consistently.")
                    .size(12),
                text("Description: free-text note included in the ingest log.").size(12),
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

        let waveform = Canvas::new(Waveform {
            data: state.waveform.clone(),
        })
        .width(Length::Fill)
        .height(Length::Fixed(260.0));

        let detection_canvas = Canvas::new(DetectionMap::new(&detection_records))
            .width(Length::Fill)
            .height(Length::Fixed(220.0));

        let detection_entries = if detection_records.is_empty() {
            Column::new().push(text("No detections to render").size(12))
        } else {
            detection_records.iter().enumerate().take(6).fold(
                Column::new().spacing(4),
                |col, (idx, detection)| {
                    col.push(
                        text(format!(
                            "#{}: range {:.1} | doppler {:.2} | SNR {:.2}",
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
            text("Power profile").size(18),
            waveform,
            text("Detection map (radius = range, angle = Doppler)").size(16),
            detection_canvas,
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
        }
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

#[derive(Clone)]
struct DetectionMap {
    records: Vec<DetectionRecord>,
}

impl DetectionMap {
    fn new(records: &[DetectionRecord]) -> Self {
        Self {
            records: records.to_vec(),
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
            Color::from_rgb(0.02, 0.02, 0.04),
        );

        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 12.0;

        for ring in 1..=3 {
            let ring_radius = radius * (ring as f32 / 3.0);
            let ring_path = Path::new(|builder| builder.circle(center, ring_radius));
            frame.stroke(
                &ring_path,
                Stroke::default().with_color(Color::from_rgb(0.25, 0.25, 0.3)),
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

        let max_range = self
            .records
            .iter()
            .map(|record| record.range)
            .fold(0.0, f32::max)
            .max(1.0);
        let max_doppler = self
            .records
            .iter()
            .map(|record| record.doppler.abs())
            .fold(0.0, f32::max)
            .max(0.5);

        for record in &self.records {
            let normalized_range = (record.range / max_range).clamp(0.0, 1.0);
            let normalized_doppler = if max_doppler > 0.0 {
                (record.doppler / max_doppler).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let point_radius = normalized_range * radius;
            let angle = normalized_doppler * PI;
            let x = center.x + point_radius * angle.cos();
            let y = center.y - point_radius * angle.sin();

            let marker_radius = 3.5 + (record.snr.min(6.0) * 0.4);
            let marker = Path::new(|builder| builder.circle(Point::new(x, y), marker_radius));
            frame.fill(&marker, Color::from_rgb(0.95, 0.55, 0.2));
        }

        vec![frame.into_geometry()]
    }
}
