//! Bevy's ordinary shaped text/UI path, kept separate from Theia's debug font.
use super::{Fixture, Run};
use bevy::{
    prelude::*,
    text::{FontSize, LineHeight},
};
use std::collections::VecDeque;

#[derive(Resource)]
pub(super) struct Stats {
    pub enabled: bool,
    pub text: String,
    pub previous_cpu_ms: f64,
    intervals: VecDeque<f64>,
}
impl Stats {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            text: String::new(),
            previous_cpu_ms: 0.,
            intervals: VecDeque::with_capacity(30),
        }
    }
    pub fn interval(&mut self, ms: f64) {
        if !self.enabled {
            return;
        }
        if self.intervals.len() == 30 {
            self.intervals.pop_front();
        }
        self.intervals.push_back(ms);
    }
    fn fps(&self) -> f64 {
        let sum: f64 = self.intervals.iter().sum();
        if sum > 0. {
            1000. * self.intervals.len() as f64 / sum
        } else {
            0.
        }
    }
}
#[derive(Component)]
pub(super) struct StatsText;

pub(super) fn setup(mut commands: Commands, stats: Res<Stats>) {
    if !stats.enabled {
        return;
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(12),
                width: px(468),
                height: px(288),
                padding: UiRect::all(px(12)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::linear_rgb(0.01, 0.018, 0.03)),
            GlobalZIndex(100),
        ))
        .with_child((
            Text::new("BEVY STATS"),
            TextFont {
                font_size: FontSize::Px(18.),
                ..default()
            },
            LineHeight::Px(24.),
            TextColor(Color::linear_rgb(0.83, 0.94, 0.95)),
            StatsText,
        ));
}
pub(super) fn update(
    fixture: Res<Fixture>,
    run: Res<Run>,
    timings: Res<super::gpu_timing::Timings>,
    probe: Res<super::stream_probe::Probe>,
    mut stats: ResMut<Stats>,
    mut text: Query<&mut Text, With<StatsText>>,
) {
    if !stats.enabled {
        return;
    }
    let frame = run.snapshot.unwrap_or(
        run.tick
            .saturating_sub(run.pre_roll)
            .min(fixture.frames.len() - 1),
    );
    let textures: usize = fixture.chunks.iter().map(|chunk| chunk.parts.len()).sum();
    let gpu = if timings.enabled && run.tick > run.pre_roll {
        timings
            .values
            .lock()
            .unwrap()
            .get(run.tick - run.pre_roll - 1)
            .and_then(|v| *v)
            .map(|v| format!("{:.3}", v.ms))
    } else {
        None
    }
    .unwrap_or_else(|| "N/A".into());
    stats.text = if probe.is_camera() {
        let state = probe.frame_report();
        let camera = &state["camera"];
        format!(
            "BEVY STATS\nFPS {:.1}\nCPU MAIN MS {:.3}\nGPU MS {}\nPROXIES {}\nREGIONS 4\nFINE {} / {}\nPUB {} EVICT {}\nENCODED KIB {:.1}\nLOGICAL KIB {:.1}\nCANCEL {} DEFER {}",
            stats.fps(),
            stats.previous_cpu_ms,
            gpu,
            fixture.players,
            fixture.frames[frame]
                .selected
                .iter()
                .filter(|&&id| id < 4)
                .count(),
            camera["slots"],
            camera["publications"],
            camera["evictions"],
            state["cpu_unique_bytes"].as_u64().unwrap() as f64 / 1024.,
            state["logical_asset_bytes"].as_u64().unwrap() as f64 / 1024.,
            camera["cancellations"],
            camera["cpu_deferrals"].as_u64().unwrap()
                + camera["logical_deferrals"].as_u64().unwrap()
        )
    } else {
        format!(
            "BEVY STATS\nFPS {:.1}\nCPU MAIN MS {:.3}\nGPU MS {}\nPROXIES {}\nREGIONS {}\nCHUNKS {} PRELOADED\nWORLD TEX {}\nSOURCE T {:.1}\nSOURCE FRAME {}\nSTREAM REPLAY",
            stats.fps(),
            stats.previous_cpu_ms,
            gpu,
            fixture.players,
            fixture.frames[frame].selected.len(),
            fixture.chunks.len(),
            textures,
            fixture.frames[frame].time,
            frame
        )
    };
    **text.single_mut().unwrap() = stats.text.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fps_uses_only_the_last_thirty_intervals_and_disabled_display_does_no_collection() {
        let mut stats = Stats::new(true);
        assert_eq!(stats.fps(), 0.);
        for _ in 0..30 {
            stats.interval(10.);
        }
        assert_eq!(stats.fps(), 100.);
        stats.interval(30.);
        assert_eq!(stats.intervals.len(), 30);
        assert_eq!(stats.fps(), 93.75);
        let mut off = Stats::new(false);
        off.interval(10.);
        assert!(off.intervals.is_empty());
        assert_eq!(off.fps(), 0.);
    }
}
