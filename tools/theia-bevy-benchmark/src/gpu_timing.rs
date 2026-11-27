//! Optional serial GPU render-graph timing. No host interval is used as GPU time.
use super::{Fixture, Run};
use bevy::{
    prelude::*,
    render::{
        Extract, ExtractSchedule, RenderApp,
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
    },
};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Resource, Clone)]
pub(super) struct Timings {
    pub enabled: bool,
    pub values: Arc<Mutex<Vec<Option<Interval>>>>,
}
#[derive(Clone, Copy)]
pub(super) struct Interval {
    pub start: u64,
    pub end: u64,
    pub ms: f64,
    pub period_ns: f64,
}
#[derive(Resource, Default)]
struct FrameTag(Option<usize>);
#[derive(Resource, Default)]
struct Recorder(Option<Queries>);
struct Queries {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    period_ns: f64,
}
impl Queries {
    fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let features =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        assert!(
            device.features().contains(features),
            "GPU timing requires timestamp queries inside encoders"
        );
        let buffer = |usage| {
            device.wgpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("comparison GPU timestamps"),
                size: 16,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            set: device
                .wgpu_device()
                .create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("comparison render graph interval"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 2,
                }),
            resolve: buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC),
            readback: buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST),
            period_ns: queue.get_timestamp_period() as f64,
        }
    }
}
pub(super) fn install(app: &mut App, enabled: bool, frames: usize) {
    let timings = Timings {
        enabled,
        values: Arc::new(Mutex::new(vec![None; if enabled { frames } else { 0 }])),
    };
    app.insert_resource(timings.clone());
    if !enabled {
        return;
    }
    app.sub_app_mut(RenderApp)
        .insert_resource(timings)
        .init_resource::<FrameTag>()
        .init_resource::<Recorder>()
        .add_systems(ExtractSchedule, extract_frame)
        .add_systems(
            RenderGraph,
            (
                begin
                    .after(RenderGraphSystems::Begin)
                    .before(RenderGraphSystems::Render),
                end.after(RenderGraphSystems::Render)
                    .before(RenderGraphSystems::Submit),
                collect.in_set(RenderGraphSystems::Finish),
            ),
        );
}
fn extract_frame(
    run: Extract<Res<Run>>,
    fixture: Extract<Res<Fixture>>,
    mut tag: ResMut<FrameTag>,
) {
    // Last increments tick before extraction. Tag the just-updated logical pose,
    // never the following main frame or the asynchronous time of readback.
    tag.0 = if run.snapshot.is_none() {
        run.tick
            .checked_sub(run.pre_roll + 1)
            .filter(|&f| f < fixture.frames.len())
    } else {
        None
    };
}
fn begin(
    tag: Res<FrameTag>,
    mut recorder: ResMut<Recorder>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut context: RenderContext,
) {
    let queries = recorder
        .0
        .get_or_insert_with(|| Queries::new(&device, &queue));
    if tag.0.is_some() {
        context.command_encoder().write_timestamp(&queries.set, 0);
    }
}
fn end(tag: Res<FrameTag>, recorder: Res<Recorder>, mut context: RenderContext) {
    if tag.0.is_none() {
        return;
    }
    let q = recorder.0.as_ref().unwrap();
    let encoder = context.command_encoder();
    encoder.write_timestamp(&q.set, 1);
    encoder.resolve_query_set(&q.set, 0..2, &q.resolve, 0);
    encoder.copy_buffer_to_buffer(&q.resolve, 0, &q.readback, 0, 16);
}
fn collect(
    tag: Res<FrameTag>,
    recorder: Res<Recorder>,
    device: Res<RenderDevice>,
    timings: Res<Timings>,
) {
    let Some(frame) = tag.0 else {
        return;
    };
    let q = recorder.0.as_ref().unwrap();
    let slice = q.readback.slice(..);
    let (sender, receiver) = mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    // Explicit serial diagnostic profile, matching the single-frame baseline.
    // This wait is outside the GPU interval but included in host cadence.
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("GPU timing completion");
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("GPU timing callback")
        .expect("GPU timing mapping");
    let mapped = slice.get_mapped_range();
    let start = u64::from_le_bytes(mapped[0..8].try_into().unwrap());
    let end = u64::from_le_bytes(mapped[8..16].try_into().unwrap());
    let ms = end.checked_sub(start).expect("GPU timestamps reversed") as f64 * q.period_ns / 1e6;
    assert!(ms.is_finite() && ms > 0., "Invalid GPU interval");
    drop(mapped);
    q.readback.unmap();
    let mut values = timings.values.lock().unwrap();
    assert!(
        values[frame]
            .replace(Interval {
                start,
                end,
                ms,
                period_ns: q.period_ns
            })
            .is_none(),
        "Duplicate GPU frame tag"
    );
}
