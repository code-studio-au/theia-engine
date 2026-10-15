//! Deterministic visual workload; proxy motion never represents gameplay authority.
use crate::{Result, camera};
use ash::vk;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Instant};

pub const STATIC_OBJECTS: u32 = 448;
pub const MAX_PLAYERS: u32 = 10_000;
pub const MAX_OBJECTS: u32 = STATIC_OBJECTS + MAX_PLAYERS;
pub const INSTANCE_OFFSET: u64 = 32 + MAX_OBJECTS as u64 * 4;
pub const WORK_BYTES: u64 = INSTANCE_OFFSET + MAX_OBJECTS as u64 * 32;

#[derive(Clone)]
pub struct Config {
    pub players: u32,
    pub overlay: bool,
    pub frames: u64,
    pub warmup: u64,
    pub seconds: f32,
    pub report: PathBuf,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.players > MAX_PLAYERS
            || !(3..=120_000).contains(&self.frames)
            || self.warmup.checked_add(2).is_none_or(|v| v >= self.frames)
            || !self.seconds.is_finite()
            || !(1.0..=600.0).contains(&self.seconds)
            || self.report.as_os_str().is_empty()
        {
            return Err("Benchmark requires players 0..10000, frames 3..120000, at least 3 measured frames, duration 1..600 seconds and a report path".into());
        }
        Ok(())
    }
    pub fn time(&self, frame: u64) -> f32 {
        self.seconds * frame.saturating_sub(self.warmup) as f32
            / (self.frames - self.warmup - 1) as f32
    }
}
pub fn rig(time: f32, seconds: f32) -> ([f32; 3], [f32; 3]) {
    let phase = (time / seconds).clamp(0., 1.) * 3.;
    let segment = phase.floor().min(2.);
    let u = phase - segment;
    let phase = segment + u * u * (3. - 2. * u);
    if phase < 1. {
        ([0., -78. + phase * 48., 12.], [0., 0., 2.])
    } else if phase < 2. {
        let angle = (phase - 1.) * std::f32::consts::TAU;
        (
            [
                angle.sin() * 30.,
                25. * (phase - 1.) - angle.cos() * 30.,
                12.,
            ],
            [0., 25. * (phase - 1.), 2.],
        )
    } else {
        let angle = (phase - 2.) * std::f32::consts::PI;
        (
            [
                angle.sin() * 30.,
                25. - angle.cos() * 30.,
                12. + (phase - 2.) * 8.,
            ],
            [0., 25., 2.],
        )
    }
}
pub fn push(
    config: &Config,
    frame: u64,
    width: u32,
    height: u32,
) -> Result<([u8; 96], [[f32; 4]; 4])> {
    let time = config.time(frame);
    let (eye, target) = rig(time, config.seconds);
    let rows = camera::perspective_rows(width, height, eye, target, 65., 0.1, 250.)?;
    let mut bytes = [0; 96];
    for (index, value) in rows
        .iter()
        .flatten()
        .chain(eye.iter())
        .chain([&time])
        .enumerate()
    {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    for (index, value) in [config.players, config.players + STATIC_OBJECTS, 0, 0]
        .iter()
        .enumerate()
    {
        bytes[80 + index * 4..84 + index * 4].copy_from_slice(&value.to_ne_bytes());
    }
    Ok((bytes, rows))
}
pub fn height(x: f32, y: f32) -> f32 {
    (x * 0.035).sin() * (y * 0.035).cos() * 1.5
}
pub fn object(id: u32, players: u32, time: f32) -> ([f32; 3], [f32; 3], u32) {
    let (x, y, extent, kind) = if id < 256 {
        (
            ((id % 16) as f32 - 7.5) * 8.,
            ((id / 16) as f32 - 7.5) * 8.,
            [4., 4., 0.5],
            0,
        )
    } else if id < 320 {
        let i = id - 256;
        (
            ((i % 8) as f32 - 3.5) * 12.,
            ((i / 8) as f32 - 3.5) * 12.,
            [2.5, 2.5, 3.],
            1,
        )
    } else if id < STATIC_OBJECTS {
        let i = id - 320;
        let a = (i.wrapping_mul(2_654_435_761) >> 16) as f32 * (std::f32::consts::TAU / 65_536.);
        (a.sin() * 55., a.cos() * 55., [0.7, 0.7, 2.], 2)
    } else {
        let i = id - STATIC_OBJECTS;
        // Bounded phase avoids large-angle f32/FMA drift at dense populations.
        let a = (i.wrapping_mul(2_654_435_761) >> 16) as f32 * (std::f32::consts::TAU / 65_536.)
            + time * (0.12 + (i % 7) as f32 * 0.01);
        let r = ((i + 1) as f32 / players.max(1) as f32).sqrt() * 22.;
        (a.cos() * r, 25. + a.sin() * r, [0.3, 0.3, 0.8], 3)
    };
    (
        [x, y, height(x, y) + if kind == 0 { 0. } else { extent[2] }],
        extent,
        kind,
    )
}
pub fn visible(rows: [[f32; 4]; 4], center: [f32; 3], extent: [f32; 3]) -> bool {
    let mut common = 63;
    for corner in 0..8 {
        // The extra top extent includes roof/head geometry for conservative rejection.
        let point = std::array::from_fn(|axis| {
            center[axis]
                + extent[axis]
                    * if axis == 2 { 1.5 } else { 1. }
                    * if corner & (1 << axis) == 0 { -1. } else { 1. }
                + if axis == 2 { extent[2] * 0.5 } else { 0. }
        });
        let [x, y, z, w] = camera::project(rows, point);
        common &= u32::from(x < -w)
            | (u32::from(x > w) << 1)
            | (u32::from(y < -w) << 2)
            | (u32::from(y > w) << 3)
            | (u32::from(z < 0.) << 4)
            | (u32::from(z > w) << 5);
    }
    common == 0
}
pub fn reference(config: &Config, frame: u64, width: u32, height_: u32) -> Result<Value> {
    let (_, rows) = push(config, frame, width, height_)?;
    let (eye, _) = rig(config.time(frame), config.seconds);
    let objects: Vec<_> = (0..STATIC_OBJECTS + config.players).map(|id| {
        let (center, extent, kind) = object(id, config.players, config.time(frame));
        let near = kind != 0 && center.iter().zip(eye).map(|(a,b)| (a-b).powi(2)).sum::<f32>() < 35.*35.;
        json!({"id":id,"center":center,"extent":extent,"kind":kind,"near":near,"visible":visible(rows,center,extent)})
    }).collect();
    Ok(json!({"time":config.time(frame),"rows":rows,"eye":eye,"objects":objects}))
}

pub struct Recorder {
    device: ash::Device,
    pool: vk::QueryPool,
    period: f32,
    mask: u64,
    pub config: Config,
    samples: Vec<Value>,
    pending: Option<(u64, f64, [u32; 2], u64, f64)>,
    started: Instant,
    last_submit: Instant,
    metadata: Value,
}
impl Recorder {
    pub fn new(
        device: &ash::Device,
        config: Config,
        period: f32,
        valid_bits: u32,
        metadata: Value,
    ) -> Result<Self> {
        config.validate()?;
        if valid_bits == 0 || valid_bits > 64 || !period.is_finite() || period <= 0. {
            return Err("Benchmark requires qualified queue timestamps".into());
        }
        // SAFETY: Device is live; this owner immediately retains the created query pool.
        let pool = unsafe {
            device.create_query_pool(
                &vk::QueryPoolCreateInfo::default()
                    .query_type(vk::QueryType::TIMESTAMP)
                    .query_count(4),
                None,
            )?
        };
        Ok(Self {
            device: device.clone(),
            pool,
            period,
            mask: u64::MAX >> (64 - valid_bits),
            config,
            samples: Vec::new(),
            pending: None,
            started: Instant::now(),
            last_submit: Instant::now(),
            metadata,
        })
    }
    /// Caller has retired the sole frame fence. No GPU-dependent work sizing is read back.
    pub fn collect(&mut self) -> Result<()> {
        if let Some((frame, cpu_ms, extent, allocated_bytes, interval_ms)) = self.pending.take() {
            let mut values = [0u64; 4];
            // SAFETY: Completed frame wrote all four queries; pool and output array are live.
            unsafe {
                self.device.get_query_pool_results(
                    self.pool,
                    0,
                    &mut values,
                    vk::QueryResultFlags::TYPE_64,
                )?;
            }
            let elapsed = |a: usize, b: usize| {
                ((values[b].wrapping_sub(values[a])) & self.mask) as f64 * self.period as f64
                    / 1_000_000.
            };
            self.samples.push(json!({"frame":frame,"warmup":frame<self.config.warmup,"scene_time":self.config.time(frame),"cpu_draw_ms":cpu_ms,"frame_interval_ms":interval_ms,"extent":extent,"tracked_image_and_work_allocation_bytes":allocated_bytes,"gpu_total_ms":elapsed(0,3),"gpu_compute_ms":elapsed(0,1),"gpu_visibility_ms":elapsed(1,2),"gpu_resolve_ms":elapsed(2,3)}));
        }
        Ok(())
    }
    pub fn begin(&self, command: vk::CommandBuffer) {
        // SAFETY: Caller retired query users and is recording this command buffer.
        unsafe {
            self.device.cmd_reset_query_pool(command, self.pool, 0, 4);
        }
        self.mark(command, 0);
    }
    pub fn mark(&self, command: vk::CommandBuffer, index: u32) {
        // SAFETY: Four indices belong to the live pool and are written once in this recording.
        unsafe {
            self.device.cmd_write_timestamp2(
                command,
                vk::PipelineStageFlags2::ALL_COMMANDS,
                self.pool,
                index,
            );
        }
    }
    pub fn submitted(&mut self, frame: u64, cpu_ms: f64, extent: [u32; 2], allocated_bytes: u64) {
        let interval = self.last_submit.elapsed().as_secs_f64() * 1000.;
        self.last_submit = Instant::now();
        self.pending = Some((frame, cpu_ms, extent, allocated_bytes, interval));
    }
    pub fn overlay(&self, frame: u64, allocated_bytes: u64) -> [u8; 32] {
        let (count, interval) = self
            .samples
            .iter()
            .rev()
            .take(30)
            .fold((0, 0.), |(n, sum), s| {
                (n + 1, sum + s["frame_interval_ms"].as_f64().unwrap())
            });
        let fps = if interval > 0. {
            1000. * count as f64 / interval
        } else {
            0.
        };
        let last = self.samples.last();
        let scaled =
            |value: f64, scale: f64| (value * scale).round().clamp(0., u32::MAX as f64) as u32;
        let time = self.config.time(frame);
        let values = [
            scaled(fps, 10.),
            scaled(
                last.map_or(0., |s| s["cpu_draw_ms"].as_f64().unwrap()),
                1000.,
            ),
            scaled(
                last.map_or(0., |s| s["gpu_total_ms"].as_f64().unwrap()),
                1000.,
            ),
            u32::from(self.config.overlay),
            ((time / self.config.seconds * 3.).floor() as u32).min(2) + 1,
            scaled(time as f64, 10.),
            scaled(allocated_bytes as f64 / 1048576., 10.),
            0,
        ];
        let mut bytes = [0; 32];
        for (i, value) in values.iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
    pub fn finish(&mut self, generations: u64) -> Result<()> {
        self.collect()?;
        if self.samples.len() as u64 != self.config.frames {
            return Err("Benchmark frame accounting incomplete".into());
        }
        let measured: Vec<_> = self
            .samples
            .iter()
            .filter(|s| s["warmup"] == false)
            .collect();
        let summary: Value = ["frame_interval_ms","cpu_draw_ms","gpu_total_ms","gpu_compute_ms","gpu_visibility_ms","gpu_resolve_ms"].into_iter().map(|key| {
            let mut values:Vec<_> = measured.iter().map(|s| s[key].as_f64().unwrap()).collect();
            values.sort_by(f64::total_cmp);
            let percentile=|p:f64| values[((values.len()-1) as f64*p).ceil() as usize];
            (key.to_string(),json!({"mean":values.iter().sum::<f64>()/values.len() as f64,"p50":percentile(0.5),"p95":percentile(0.95),"p99":percentile(0.99),"max":values[values.len()-1]}))
        }).collect::<serde_json::Map<_,_>>().into();
        if let Some(parent) = self
            .config
            .report
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let report = json!({"schema":1,"workload":"theia-procedural-mmo-v1","players":self.config.players,"static_objects":STATIC_OBJECTS,"frames":self.config.frames,"warmup_frames":self.config.warmup,"logical_duration_seconds":self.config.seconds,"wall_seconds":self.started.elapsed().as_secs_f64(),"presentation":"FIFO; single frame in flight","cpu_metric":"draw call wall time including fence/acquire/present; excludes event loop gaps","gpu_metric":"four ALL_COMMANDS timestamps on one graphics/compute queue; instrumentation enabled","overlay":self.config.overlay,"streaming":false,"authority":false,"skinned_animation":false,"generations":generations,"metadata":self.metadata,"summary_ms":summary,"samples":self.samples});
        std::fs::write(&self.config.report, serde_json::to_vec_pretty(&report)?)?;
        println!(
            "Benchmark report written: {} ({} measured frames)",
            self.config.report.display(),
            measured.len()
        );
        Ok(())
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        // SAFETY: Renderer retires frame work before dropping this owner; context device stays alive.
        unsafe {
            self.device.destroy_query_pool(self.pool, None);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_edges_match_and_bounds_cover_all_surface_corners() {
        for id in 0..256 {
            let (center, extent, kind) = object(id, 0, 0.);
            assert_eq!(kind, 0);
            for dx in [-4., 4.] {
                for dy in [-4., 4.] {
                    let z = height(center[0] + dx, center[1] + dy);
                    assert!(z >= center[2] - extent[2] && z <= center[2] + extent[2] * 2.);
                }
            }
            if id % 16 < 15 {
                let (other, _, _) = object(id + 1, 0, 0.);
                for dy in [-4., 4.] {
                    assert_eq!(
                        height(center[0] + 4., center[1] + dy).to_bits(),
                        height(other[0] - 4., other[1] + dy).to_bits()
                    );
                }
            }
            if id / 16 < 15 {
                let (other, _, _) = object(id + 16, 0, 0.);
                for dx in [-4., 4.] {
                    assert_eq!(
                        height(center[0] + dx, center[1] + 4.).to_bits(),
                        height(other[0] + dx, other[1] - 4.).to_bits()
                    );
                }
            }
        }
    }
    #[test]
    fn invalid_population_timing_and_overflowing_warmup_are_rejected() {
        let valid = Config {
            overlay: true,
            players: 0,
            frames: 3,
            warmup: 0,
            seconds: 60.,
            report: "unused.json".into(),
        };
        valid.validate().unwrap();
        for invalid in [
            Config {
                players: MAX_PLAYERS + 1,
                ..valid.clone()
            },
            Config {
                frames: 2,
                ..valid.clone()
            },
            Config {
                warmup: u64::MAX,
                ..valid.clone()
            },
            Config {
                seconds: f32::NAN,
                ..valid.clone()
            },
            Config {
                seconds: 0.,
                ..valid.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
    #[test]
    fn timeline_covers_all_phases_and_warmup_is_stationary() {
        let c = Config {
            overlay: true,
            players: 100,
            frames: 181,
            warmup: 1,
            seconds: 60.,
            report: "unused.json".into(),
        };
        c.validate().unwrap();
        assert_eq!(c.time(0), 0.);
        assert_eq!(c.time(1), 0.);
        assert_eq!(c.time(180), 60.);
        for frame in 0..181 {
            assert!(
                push(&c, frame, 960, 540)
                    .unwrap()
                    .1
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite())
            );
        }
        let (a, _) = rig(20. - 0.0001, 60.);
        let (b, _) = rig(20. + 0.0001, 60.);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.001));
        let (a, _) = rig(40. - 0.0001, 60.);
        let (b, _) = rig(40. + 0.0001, 60.);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.001));
    }
    #[test]
    fn proxy_motion_is_finite_deterministic_and_within_bounded_scene() {
        for id in 0..MAX_OBJECTS {
            for time in [0., 20., 60.] {
                let (p, e, kind) = object(id, MAX_PLAYERS, time);
                assert!(p.into_iter().chain(e).all(f32::is_finite));
                assert!(p[0].abs() <= 64. && p[1].abs() <= 64.);
                assert!(kind <= 3);
            }
        }
        assert_ne!(
            object(STATIC_OBJECTS, 100, 0.).0,
            object(STATIC_OBJECTS, 100, 20.).0
        );
        assert_eq!(object(256, 100, 0.), object(256, 100, 20.));
    }
}
