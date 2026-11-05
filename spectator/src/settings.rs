//! The options of the window and how they are kept between runs.

use core::{ops::RangeInclusive, time::Duration};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Allowed seconds between two rounds on display.
pub const ROUND_DISPLAY_SECONDS: RangeInclusive<u64> = 1..=300;
/// Allowed seconds the progress bar takes.
pub const PROGRESS_SECONDS: RangeInclusive<u64> = 1..=300;
/// Allowed seconds between two samples of the scores.
pub const SAMPLE_SECONDS: RangeInclusive<u64> = 1..=300;
/// Allowed seconds the points per minute look back.
pub const SMOOTHING_SECONDS: RangeInclusive<u64> = 10..=3600;
/// Allowed seconds the charts show.
pub const SHOWN_SPAN_SECONDS: RangeInclusive<u64> = 30..=12 * 3600;
/// Allowed zoom factors of the window.
pub const ZOOM: RangeInclusive<f32> = 0.5..=3.0;

const FILE_NAME: &str = "settings.conf";
const DIRECTORY: &str = "maexchen-spectator";

/// Everything the audience or the host can change while the spectator runs.
///
/// The defaults are the values of the Java spectator.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The server runs with half Mia: 4,2 is Mäxchen and 2,1 only half of it.
    pub half_mia: bool,
    /// How long a finished round stays on display.
    pub round_display: Duration,
    /// Over how many seconds the progress bar towards the next round runs.
    pub progress: Duration,
    /// How often the scores are sampled for the charts.
    pub sample: Duration,
    /// Sample as often as rounds are shown instead of using [`Settings::sample`].
    pub sample_follows_round: bool,
    /// How far back the points per minute look.
    pub smoothing: Duration,
    /// How much of the history the charts show.
    pub shown_span: Duration,
    /// Scales the whole window.
    pub zoom: f32,
    /// The window covers the whole screen.
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            half_mia: false,
            round_display: Duration::from_secs(10),
            progress: Duration::from_secs(10),
            sample: Duration::from_secs(10),
            sample_follows_round: false,
            smoothing: Duration::from_secs(50),
            shown_span: Duration::from_secs(300),
            zoom: 1.0,
            fullscreen: false,
        }
    }
}

impl Settings {
    /// The time between two samples, taking [`Settings::sample_follows_round`] into account.
    #[must_use]
    pub fn effective_sample(&self) -> Duration {
        if self.sample_follows_round {
            self.round_display
        } else {
            self.sample
        }
    }

    /// Reads settings from `key=value` lines; anything missing or invalid keeps its default and
    /// numbers are limited to their allowed range.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
            let value = value.trim();
            match key.trim() {
                "half_mia" => set(&mut settings.half_mia, value.parse().ok()),
                "round_display" => set(
                    &mut settings.round_display,
                    seconds(value, &ROUND_DISPLAY_SECONDS),
                ),
                "progress" => set(&mut settings.progress, seconds(value, &PROGRESS_SECONDS)),
                "sample" => set(&mut settings.sample, seconds(value, &SAMPLE_SECONDS)),
                "sample_follows_round" => {
                    set(&mut settings.sample_follows_round, value.parse().ok());
                }
                "smoothing" => set(&mut settings.smoothing, seconds(value, &SMOOTHING_SECONDS)),
                "shown_span" => set(
                    &mut settings.shown_span,
                    seconds(value, &SHOWN_SPAN_SECONDS),
                ),
                "zoom" => set(&mut settings.zoom, zoom(value)),
                "fullscreen" => set(&mut settings.fullscreen, value.parse().ok()),
                _ => {}
            }
        }
        settings
    }

    /// Writes the settings the way [`Settings::parse`] reads them.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            "half_mia={}\nround_display={}\nprogress={}\nsample={}\nsample_follows_round={}\n\
             smoothing={}\nshown_span={}\nzoom={}\nfullscreen={}\n",
            self.half_mia,
            self.round_display.as_secs(),
            self.progress.as_secs(),
            self.sample.as_secs(),
            self.sample_follows_round,
            self.smoothing.as_secs(),
            self.shown_span.as_secs(),
            self.zoom,
            self.fullscreen,
        )
    }

    /// Reads the settings from `path`; the defaults if there is no file or it cannot be read.
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                log::warn!("cannot read {}: {error}", path.display());
                Self::default()
            }
        }
    }

    /// Writes the settings to `path` and creates the directory if needed.
    ///
    /// # Errors
    ///
    /// Fails if the directory or the file cannot be written.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        fs::write(path, self.to_text())
    }

    /// Reads the settings of the last run from the configuration directory of the user.
    #[must_use]
    pub fn load() -> Self {
        default_path().map_or_else(Self::default, |path| Self::load_from(&path))
    }

    /// Keeps the settings for the next run in the configuration directory of the user.
    ///
    /// # Errors
    ///
    /// Fails if there is no configuration directory or the file cannot be written.
    pub fn save(&self) -> io::Result<()> {
        let path = default_path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no configuration directory found")
        })?;
        self.save_to(&path)
    }
}

fn set<T>(target: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *target = value;
    }
}

fn seconds(text: &str, allowed: &RangeInclusive<u64>) -> Option<Duration> {
    let seconds: u64 = text.parse().ok()?;
    Some(Duration::from_secs(
        seconds.clamp(*allowed.start(), *allowed.end()),
    ))
}

fn zoom(text: &str) -> Option<f32> {
    let factor: f32 = text.parse().ok()?;
    factor
        .is_finite()
        .then(|| factor.clamp(*ZOOM.start(), *ZOOM.end()))
}

fn default_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .or_else(|| env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join(DIRECTORY).join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::{env, fs, path::PathBuf};

    use super::{DIRECTORY, FILE_NAME, Settings, default_path};

    fn changed() -> Settings {
        Settings {
            half_mia: true,
            round_display: Duration::from_secs(20),
            progress: Duration::from_secs(7),
            sample: Duration::from_secs(15),
            sample_follows_round: true,
            smoothing: Duration::from_secs(120),
            shown_span: Duration::from_secs(60),
            zoom: 1.5,
            fullscreen: true,
        }
    }

    fn scratch_file(name: &str) -> PathBuf {
        env::temp_dir()
            .join(format!("maexchen-spectator-{}-{name}", std::process::id()))
            .join("settings.conf")
    }

    #[test]
    fn settings_survive_writing_and_reading() {
        assert_eq!(Settings::parse(&changed().to_text()), changed());
    }

    #[test]
    fn missing_keys_keep_their_defaults() {
        assert_eq!(Settings::parse(""), Settings::default());
        assert_eq!(
            Settings::parse("half_mia=true\n"),
            Settings {
                half_mia: true,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn unknown_keys_and_garbage_are_ignored() {
        let text = "colour=blue\nround_display=soon\nzoom=NaN\nno equals sign\nfullscreen=maybe\n";

        assert_eq!(Settings::parse(text), Settings::default());
    }

    #[test]
    fn numbers_are_kept_within_their_range() {
        let settings = Settings::parse("round_display=0\nsmoothing=999999\nzoom=9\nprogress=0\n");

        assert_eq!(settings.round_display, Duration::from_secs(1));
        assert_eq!(settings.smoothing, Duration::from_secs(3600));
        assert_eq!(settings.progress, Duration::from_secs(1));
        assert!((settings.zoom - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn samples_follow_the_round_display_on_request() {
        let mut settings = Settings {
            round_display: Duration::from_secs(30),
            sample: Duration::from_secs(5),
            ..Settings::default()
        };
        assert_eq!(settings.effective_sample(), Duration::from_secs(5));

        settings.sample_follows_round = true;
        assert_eq!(settings.effective_sample(), Duration::from_secs(30));
    }

    #[test]
    fn settings_are_kept_in_a_file() {
        let path = scratch_file("keep");

        changed().save_to(&path).expect("the file can be written");

        assert_eq!(Settings::load_from(&path), changed());
        fs::remove_dir_all(path.parent().expect("the file has a directory")).ok();
    }

    #[test]
    fn a_file_that_cannot_be_read_leaves_the_defaults() {
        let directory = scratch_file("unreadable");
        let directory = directory.parent().expect("the file has a directory");
        fs::create_dir_all(directory).expect("the directory can be created");

        assert_eq!(Settings::load_from(directory), Settings::default());
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn a_file_in_the_place_of_a_directory_cannot_be_kept() {
        let blocker = scratch_file("blocked");
        let directory = blocker.parent().expect("the file has a directory");
        fs::create_dir_all(directory).expect("the directory can be created");
        let in_a_file = directory.join("settings.conf").join("nested.conf");
        fs::write(directory.join("settings.conf"), "").expect("the file can be written");

        assert!(Settings::default().save_to(&in_a_file).is_err());
        fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn the_options_live_in_the_configuration_directory_of_the_user() {
        if let Some(path) = default_path() {
            assert!(path.ends_with(PathBuf::from(DIRECTORY).join(FILE_NAME)));
        }
    }

    #[test]
    fn without_a_file_the_defaults_apply() {
        assert_eq!(
            Settings::load_from(&scratch_file("missing")),
            Settings::default()
        );
    }
}
