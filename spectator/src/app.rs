//! The window: round on the left, scores and charts on the right, options at the bottom.

use core::{ops::RangeInclusive, time::Duration};
use std::{sync::mpsc::Receiver, time::Instant};

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::{
    board::RoundBoard,
    message::Message,
    model::GameView,
    settings::{
        PROGRESS_SECONDS, ROUND_DISPLAY_SECONDS, SAMPLE_SECONDS, SHOWN_SPAN_SECONDS,
        SMOOTHING_SECONDS, Settings, ZOOM,
    },
    texts::{Rules, decimal, grouped},
    update::{Status, Update},
};

const REPAINT_INTERVAL: Duration = Duration::from_millis(250);
const SECONDS_PER_MINUTE: f64 = 60.0;
/// Room above the highest and below the lowest value, as a share of the spread between them.
const Y_PADDING: f64 = 0.1;
/// Room around values that are all the same, as a share of that value.
const FLAT_PADDING: f64 = 0.05;
const WARNING: Color32 = Color32::from_rgb(0xff, 0xa0, 0x30);

type Series = (String, Vec<[f64; 2]>);

/// What the chart on the right shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chart {
    PointsPerMinute,
    Score,
}

/// The state shown by the window.
struct SpectatorApp {
    updates: Receiver<Update>,
    started: Instant,
    server: String,
    view: GameView,
    board: RoundBoard,
    status: Status,
    chart: Chart,
    settings: Settings,
}

/// Opens the window and shows what arrives on `updates` until the window is closed; `server`
/// (`host:port`) is shown at the bottom.
///
/// The options of the last run are restored and kept again when the window is closed.
///
/// # Errors
///
/// Fails if the window cannot be opened.
#[cfg(not(tarpaulin_include))]
pub fn run(server: String, updates: Receiver<Update>) -> eframe::Result {
    let settings = Settings::load();
    let window = egui::ViewportBuilder::default()
        .with_title("Mäxchen!")
        .with_app_id("maexchen-spectator")
        .with_inner_size([1100.0, 700.0])
        .with_fullscreen(settings.fullscreen);
    eframe::run_native(
        "Mäxchen!",
        eframe::NativeOptions {
            viewport: window,
            ..eframe::NativeOptions::default()
        },
        Box::new(move |creation| {
            creation.egui_ctx.set_theme(egui::Theme::Dark);
            creation.egui_ctx.set_zoom_factor(settings.zoom);
            Ok(Box::new(SpectatorApp::new(server, updates, settings)))
        }),
    )
}

impl SpectatorApp {
    fn new(server: String, updates: Receiver<Update>, settings: Settings) -> Self {
        Self {
            updates,
            started: Instant::now(),
            server,
            view: GameView::default(),
            board: RoundBoard::default(),
            status: Status::Connected,
            chart: Chart::PointsPerMinute,
            settings,
        }
    }

    fn rules(&self) -> Rules {
        Rules {
            half_mia: self.settings.half_mia,
        }
    }

    fn take_updates(&mut self) {
        let now = self.started.elapsed();
        let arrived: Vec<Update> = self.updates.try_iter().collect();
        for update in arrived {
            match update {
                Update::Message(message) => self.apply(message, now),
                Update::Status(status) => self.status = status,
            }
        }
    }

    fn apply(&mut self, message: Message, at: Duration) {
        self.board.apply(&message, at);
        self.view.apply(message, at);
    }

    fn follow_settings(&mut self, ctx: &egui::Context) {
        self.board.set_throttle(self.settings.round_display);
        self.view
            .set_sample_interval(self.settings.effective_sample());
        self.settings.zoom = ctx.zoom_factor();
        if let Some(fullscreen) = ctx.input(|input| input.viewport().fullscreen) {
            self.settings.fullscreen = fullscreen;
        }
    }

    fn show_round(&self, ui: &mut egui::Ui) {
        match self.board.shown() {
            None => ui.heading("Warte auf erste Runde..."),
            Some(report) => ui.heading(format!("Ablauf von Runde {}", grouped(report.number))),
        };
        egui::TopBottomPanel::bottom("next-round").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("nächste Runde:");
                let progress = self
                    .board
                    .progress(self.started.elapsed(), self.settings.progress);
                ui.add(egui::ProgressBar::new(progress).desired_width(ui.available_width()));
            });
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(report) = self.board.shown() {
                for line in report.lines(self.rules()) {
                    ui.label(line);
                }
            }
        });
    }

    fn show_scores(&self, ui: &mut egui::Ui) {
        let mut scores = self.view.scores().to_vec();
        scores.sort_by(|(name_a, points_a), (name_b, points_b)| {
            points_b.cmp(points_a).then_with(|| name_a.cmp(name_b))
        });
        match (scores.is_empty(), self.view.scores_round()) {
            (true, _) => ui.heading("Warte auf ersten Bot..."),
            (false, None) => ui.heading("Punktestand"),
            (false, Some(round)) => {
                ui.heading(format!("Punktestand nach Runde {}", grouped(round)))
            }
        };
        egui::Grid::new("scores").striped(true).show(ui, |ui| {
            ui.strong("Name");
            ui.strong("Punkte");
            ui.end_row();
            for (name, points) in scores {
                ui.label(name);
                ui.label(grouped(points));
                ui.end_row();
            }
        });
    }

    fn show_chart(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.chart, Chart::PointsPerMinute, "Punkte pro Minute");
            ui.selectable_value(&mut self.chart, Chart::Score, "Punktestand");
        });
        let from = self
            .started
            .elapsed()
            .saturating_sub(self.settings.shown_span);
        let to = from + self.settings.shown_span;
        let series = self.chart_series(from);
        let heights = padded_range(&series);
        let unit = match self.chart {
            Chart::PointsPerMinute => "Punkte pro Minute",
            Chart::Score => "Punkte",
        };
        let plot = match self.chart {
            Chart::PointsPerMinute => Plot::new("points-per-minute"),
            Chart::Score => Plot::new("score"),
        };
        plot.legend(Legend::default())
            .x_axis_formatter(|mark, _range| format_minutes(mark.value))
            .y_axis_formatter(|mark, _range| decimal(mark.value))
            .label_formatter(move |name, point| hover_label(name, point.x, point.y, unit))
            .height(ui.available_height())
            .show(ui, |plot| {
                plot.set_plot_bounds_x(minutes(from)..=minutes(to));
                match heights {
                    Some(range) => plot.set_plot_bounds_y(range),
                    None => plot.set_auto_bounds([false, true]),
                }
                for (name, points) in series {
                    plot.line(Line::new(name, PlotPoints::from(points)));
                }
            });
    }

    fn chart_series(&self, from: Duration) -> Vec<Series> {
        match self.chart {
            Chart::PointsPerMinute => self
                .view
                .points_per_minute_series(self.settings.smoothing)
                .into_iter()
                .map(|(name, rates)| (name, plot_points(rates.into_iter(), from)))
                .collect(),
            Chart::Score => self
                .view
                .history()
                .iter()
                .map(|(name, samples)| {
                    let points = samples.iter().map(|(at, points)| (*at, f64::from(*points)));
                    (name.clone(), plot_points(points, from))
                })
                .collect(),
        }
    }

    fn show_options(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Optionen").show(ui, |ui| self.show_option_fields(ui));
    }

    fn show_option_fields(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.settings.half_mia, "Halbes Mäxchen Regeln")
            .on_hover_text("(4,2) = Mäxchen, (2,1) = halbes Mäxchen");

        ui.separator();
        ui.strong("Runde");
        egui::Grid::new("round-options").show(ui, |ui| {
            seconds_row(
                ui,
                "Rundenanzeige alle",
                &mut self.settings.round_display,
                ROUND_DISPLAY_SECONDS,
            );
            seconds_row(
                ui,
                "Fortschrittsbalken über",
                &mut self.settings.progress,
                PROGRESS_SECONDS,
            );
        });

        ui.separator();
        ui.strong("Diagramme");
        egui::Grid::new("chart-options").show(ui, |ui| {
            ui.label("Messpunkt alle");
            ui.horizontal(|ui| {
                let follows = self.settings.sample_follows_round;
                if follows {
                    let mut shown = self.settings.round_display.as_secs();
                    ui.add_enabled(false, seconds_field(&mut shown, SAMPLE_SECONDS));
                } else {
                    let mut sample = self.settings.sample.as_secs();
                    ui.add(seconds_field(&mut sample, SAMPLE_SECONDS));
                    self.settings.sample = Duration::from_secs(sample);
                }
                ui.checkbox(&mut self.settings.sample_follows_round, "wie Rundenanzeige");
            });
            ui.end_row();
            seconds_row(
                ui,
                "Glättung über",
                &mut self.settings.smoothing,
                SMOOTHING_SECONDS,
            );
            seconds_row(
                ui,
                "Verlauf der letzten",
                &mut self.settings.shown_span,
                SHOWN_SPAN_SECONDS,
            );
        });

        ui.separator();
        ui.strong("Fenster");
        let mut zoom = self.settings.zoom;
        if ui
            .add(egui::Slider::new(&mut zoom, ZOOM).text("Zoom"))
            .changed()
        {
            ui.ctx().set_zoom_factor(zoom);
        }
        let mut fullscreen = self.settings.fullscreen;
        if ui.checkbox(&mut fullscreen, "Vollbild (F11)").changed() {
            set_fullscreen(ui.ctx(), fullscreen);
        }
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Server: {}", self.server));
            if let Status::Reconnecting { reason, retry_in } = &self.status {
                ui.label(
                    RichText::new(format!(
                        "Verbindung verloren ({reason}), neuer Versuch in {} s",
                        retry_in.as_secs()
                    ))
                    .color(WARNING),
                );
            }
        });
    }
}

impl SpectatorApp {
    fn show_frame(&mut self, ctx: &egui::Context) {
        self.take_updates();
        ctx.request_repaint_after(REPAINT_INTERVAL);

        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            set_fullscreen(ctx, !self.settings.fullscreen);
        }
        self.follow_settings(ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.show_options(ui);
            self.show_status(ui);
        });
        egui::SidePanel::left("round")
            .default_width(480.0)
            .show(ctx, |ui| self.show_round(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_scores(ui);
            ui.separator();
            self.show_chart(ui);
        });
    }
}

impl eframe::App for SpectatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_frame(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(error) = self.settings.save() {
            log::warn!("cannot keep the options: {error}");
        }
    }
}

fn set_fullscreen(ctx: &egui::Context, fullscreen: bool) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
}

fn seconds_field(seconds: &mut u64, allowed: RangeInclusive<u64>) -> egui::DragValue<'_> {
    egui::DragValue::new(seconds).range(allowed).suffix(" s")
}

fn seconds_row(ui: &mut egui::Ui, label: &str, value: &mut Duration, allowed: RangeInclusive<u64>) {
    ui.label(label);
    let mut seconds = value.as_secs();
    ui.add(seconds_field(&mut seconds, allowed));
    *value = Duration::from_secs(seconds);
    ui.end_row();
}

fn plot_points(points: impl Iterator<Item = (Duration, f64)>, from: Duration) -> Vec<[f64; 2]> {
    points
        .filter(|(at, _)| *at >= from)
        .map(|(at, value)| [minutes(at), value])
        .collect()
}

/// The heights the charts show: from the lowest to the highest value in view with some room, or
/// `None` while there are fewer than two values to fit.
fn padded_range(series: &[Series]) -> Option<RangeInclusive<f64>> {
    let values: Vec<f64> = series
        .iter()
        .flat_map(|(_, points)| points.iter().map(|point| point[1]))
        .collect();
    if values.len() < 2 {
        return None;
    }
    let (lowest, highest) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &value| {
            (low.min(value), high.max(value))
        });
    let spread = highest - lowest;
    let room = if spread > 0.0 {
        spread * Y_PADDING
    } else {
        (highest.abs() * FLAT_PADDING).max(1.0)
    };
    Some(lowest - room..=highest + room)
}

fn minutes(duration: Duration) -> f64 {
    duration.as_secs_f64() / SECONDS_PER_MINUTE
}

/// The text next to the pointer: the player if it is over a line, the time as `m:ss` and the value.
fn hover_label(player: &str, minutes: f64, value: f64, unit: &str) -> String {
    let reading = format!(
        "{}
{} {unit}",
        format_minutes(minutes),
        decimal(value)
    );
    if player.is_empty() {
        reading
    } else {
        format!(
            "{player}
{reading}"
        )
    }
}

/// Writes minutes since the start of the spectator as `m:ss`.
fn format_minutes(minutes: f64) -> String {
    let seconds = Duration::try_from_secs_f64(minutes.max(0.0) * SECONDS_PER_MINUTE)
        .map_or(0, |duration| duration.as_secs());
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::sync::mpsc::{self, Sender};

    use eframe::egui;

    use super::{Chart, Series, SpectatorApp, format_minutes, hover_label, padded_range};
    use crate::{
        message::{CancelReason, Dice, LossReason, Message},
        settings::Settings,
        update::{Status, Update},
    };

    fn app() -> (SpectatorApp, Sender<Update>) {
        let (sender, receiver) = mpsc::channel();
        let app = SpectatorApp::new("localhost:9000".to_string(), receiver, Settings::default());
        (app, sender)
    }

    fn texts_of(shape: &egui::Shape, found: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                found.push_str(text.galley.text());
                found.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts_of(shape, found)),
            _ => {}
        }
    }

    fn shown_text(output: &egui::FullOutput) -> String {
        let mut found = String::new();
        for clipped in &output.shapes {
            texts_of(&clipped.shape, &mut found);
        }
        found
    }

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1100.0, 700.0),
            )),
            events,
            ..egui::RawInput::default()
        }
    }

    fn frame_with(app: &mut SpectatorApp, events: Vec<egui::Event>) -> egui::FullOutput {
        let ctx = egui::Context::default();
        ctx.run(input(events), |ctx| app.show_frame(ctx))
    }

    fn frame(app: &mut SpectatorApp) -> egui::FullOutput {
        frame_with(app, vec![])
    }

    fn names(players: &[&str]) -> Vec<String> {
        players.iter().map(ToString::to_string).collect()
    }

    fn play_a_round(app: &mut SpectatorApp, number: u64, start: u64) {
        let at = |seconds: u64| Duration::from_secs(start + seconds);
        app.apply(
            Message::RoundStarted {
                number,
                players: names(&["anna", "bert"]),
                rules: None,
            },
            at(0),
        );
        app.apply(Message::PlayerRolls("anna".to_string()), at(1));
        app.apply(
            Message::Announced {
                player: "anna".to_string(),
                dice: Dice { high: 5, low: 3 },
            },
            at(2),
        );
        app.apply(Message::PlayerWantsToSee("bert".to_string()), at(3));
        app.apply(Message::ActualDice(Dice { high: 4, low: 1 }), at(4));
        app.apply(
            Message::PlayerLost {
                players: names(&["anna"]),
                reason: LossReason::CaughtBluffing,
            },
            at(5),
        );
        app.apply(
            Message::Score(vec![("anna".to_string(), 0), ("bert".to_string(), 1)]),
            at(6),
        );
    }

    #[test]
    fn an_empty_window_waits_for_the_first_round_and_the_first_bot() {
        let (mut app, _sender) = app();

        let text = shown_text(&frame(&mut app));

        assert!(text.contains("Warte auf erste Runde..."), "{text}");
        assert!(text.contains("Warte auf ersten Bot..."), "{text}");
        assert!(text.contains("Server: localhost:9000"), "{text}");
        assert!(!text.contains("Verbindung verloren"), "{text}");
    }

    #[test]
    fn the_scores_are_listed_best_first_after_the_round_they_belong_to() {
        let (mut app, _sender) = app();
        play_a_round(&mut app, 1_234, 0);
        play_a_round(&mut app, 1_235, 10);

        let text = shown_text(&frame(&mut app));

        assert!(text.contains("Punktestand nach Runde 1.235"), "{text}");
        let table = text
            .split_once("Punktestand nach Runde")
            .expect("the heading is there")
            .1;
        let bert = table.find("bert").expect("bert is listed");
        let anna = table.find("anna").expect("anna is listed");
        assert!(bert < anna, "{text}");
    }

    #[test]
    fn updates_from_the_connection_reach_the_window() {
        let (mut app, sender) = app();
        sender
            .send(Update::Message(Message::RoundCanceled(
                CancelReason::OnlyOnePlayer,
            )))
            .expect("the window listens");
        sender
            .send(Update::Status(Status::Reconnecting {
                reason: "weg".to_string(),
                retry_in: Duration::from_secs(5),
            }))
            .expect("the window listens");

        let text = shown_text(&frame(&mut app));

        assert!(
            text.contains("Verbindung verloren (weg), neuer Versuch in 5 s"),
            "{text}"
        );

        sender
            .send(Update::Status(Status::Connected))
            .expect("the window listens");
        assert!(!shown_text(&frame(&mut app)).contains("Verbindung verloren"));
    }

    #[test]
    fn both_charts_show_their_players() {
        let (mut app, _sender) = app();
        play_a_round(&mut app, 1, 0);
        play_a_round(&mut app, 2, 60);

        for chart in [Chart::PointsPerMinute, Chart::Score] {
            app.chart = chart;
            let text = shown_text(&frame(&mut app));
            assert!(text.contains("Punkte pro Minute"), "{text}");
            assert!(text.contains("Punktestand"), "{text}");
        }
    }

    #[test]
    fn the_window_follows_the_options() {
        let (mut app, _sender) = app();
        app.settings.round_display = Duration::from_secs(30);

        frame(&mut app);

        assert_eq!(app.board.throttle(), Duration::from_secs(30));
        assert!((app.settings.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn f11_switches_to_full_screen() {
        let (mut app, _sender) = app();
        let f11 = egui::Event::Key {
            key: egui::Key::F11,
            physical_key: Some(egui::Key::F11),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };

        let output = frame_with(&mut app, vec![f11]);

        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Fullscreen(true))),
            "{commands:?}"
        );
    }

    #[test]
    fn the_options_show_the_sampling_either_fixed_or_free() {
        for follows_round in [true, false] {
            let (mut app, _sender) = app();
            app.settings.sample_follows_round = follows_round;
            let ctx = egui::Context::default();

            let output = ctx.run(input(vec![]), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.show_option_fields(ui));
            });

            let text = shown_text(&output);
            assert!(text.contains("Messpunkt alle"), "{text}");
            assert!(text.contains("Vollbild (F11)"), "{text}");
            assert_eq!(app.settings.sample_follows_round, follows_round);
        }
    }

    #[test]
    fn the_options_are_folded_away_until_opened() {
        let (mut app, _sender) = app();

        let text = shown_text(&frame(&mut app));

        assert!(text.contains("Optionen"), "{text}");
        assert!(!text.contains("Messpunkt alle"), "{text}");
    }

    #[test]
    fn the_tooltip_shows_time_and_points_formatted() {
        assert_eq!(
            hover_label("anna", 2.5, 20_123.0, "Punkte"),
            "anna\n2:30\n20.123 Punkte"
        );
    }

    #[test]
    fn the_tooltip_has_no_name_away_from_the_lines() {
        assert_eq!(
            hover_label("", 0.25, 1234.5, "Punkte pro Minute"),
            "0:15\n1.234,5 Punkte pro Minute"
        );
    }

    fn series(values: &[&[f64]]) -> Vec<Series> {
        values
            .iter()
            .map(|heights| {
                let points = heights
                    .iter()
                    .zip(0..)
                    .map(|(&height, at)| [f64::from(at), height]);
                ("anna".to_string(), points.collect())
            })
            .collect()
    }

    #[test]
    fn the_range_fits_the_values_with_room_and_needs_no_zero() {
        let range = padded_range(&series(&[&[100.0, 110.0], &[104.0]])).expect("three values");

        assert!((*range.start() - 99.0).abs() < 1e-9);
        assert!((*range.end() - 111.0).abs() < 1e-9);
    }

    #[test]
    fn nothing_is_fitted_before_two_values_are_there() {
        assert_eq!(padded_range(&series(&[])), None);
        assert_eq!(padded_range(&series(&[&[5.0]])), None);
    }

    #[test]
    fn equal_values_get_room_as_well() {
        let range = padded_range(&series(&[&[200.0, 200.0]])).expect("two values");

        assert!((*range.start() - 190.0).abs() < 1e-9);
        assert!((*range.end() - 210.0).abs() < 1e-9);

        let zero = padded_range(&series(&[&[0.0, 0.0]])).expect("two values");
        assert!((*zero.start() + 1.0).abs() < 1e-9);
        assert!((*zero.end() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn minutes_are_written_with_seconds() {
        assert_eq!(format_minutes(0.0), "0:00");
        assert_eq!(format_minutes(2.5), "2:30");
        assert_eq!(format_minutes(61.0), "61:00");
    }

    #[test]
    fn odd_values_do_not_break_the_axis() {
        assert_eq!(format_minutes(-3.0), "0:00");
        assert_eq!(format_minutes(f64::NAN), "0:00");
        assert_eq!(format_minutes(f64::INFINITY), "0:00");
    }
}
