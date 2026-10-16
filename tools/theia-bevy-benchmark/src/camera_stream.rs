//! Repeated demand with one operation in flight. All asset preparation/removal
//! acknowledgements use the fine-probe's verified render-world watch.
use super::*;
use std::collections::VecDeque;
use theia_assets::world_demand::Policy;

pub(super) struct Camera {
    policy: Policy,
    blocked: [Option<u64>; 4],
    pub ending: bool,
    pub capture_frames: Vec<usize>,
    pub plan: serde_json::Value,
    history: VecDeque<serde_json::Value>,
    dropped: u64,
    uploads: u64,
    requests: u64,
    publications: u64,
    evictions: u64,
    cancellations: u64,
    worker_cancellations: u64,
    ready_cancellations: u64,
    cpu_deferrals: u64,
    logical_deferrals: u64,
}
impl Camera {
    pub fn new(slots: usize, capture_frames: Vec<usize>) -> Self {
        Self {
            policy: Policy {
                load_radius: 20.,
                retain_radius: 35.,
                slots,
            },
            blocked: [None; 4],
            ending: false,
            capture_frames,
            plan: serde_json::Value::Null,
            history: VecDeque::new(),
            dropped: 0,
            uploads: 0,
            requests: 0,
            publications: 0,
            evictions: 0,
            cancellations: 0,
            worker_cancellations: 0,
            ready_cancellations: 0,
            cpu_deferrals: 0,
            logical_deferrals: 0,
        }
    }
    fn event(&mut self, probe: &Probe, frame: usize, kind: &str) {
        let mut row = probe.state_report();
        row["frame"] = frame.into();
        row["kind"] = kind.into();
        row["ending"] = self.ending.into();
        if self.history.len() == 64 {
            self.history.pop_front();
            self.dropped += 1;
        }
        self.history.push_back(row);
    }
    pub fn report(&self, history: bool) -> serde_json::Value {
        let mut row = serde_json::json!({"slots":self.policy.slots,"load_radius":20,"retain_radius":35,"ending":self.ending,"plan":self.plan,"requests":self.requests,"uploads":self.uploads,"publications":self.publications,"evictions":self.evictions,"cancellations":self.cancellations,"worker_cancellations":self.worker_cancellations,"ready_cancellations":self.ready_cancellations,"cpu_deferrals":self.cpu_deferrals,"logical_deferrals":self.logical_deferrals,"dropped_history":self.dropped});
        if history {
            row["history"] = serde_json::json!(self.history);
        }
        row
    }
}
pub(super) fn tick(
    probe: &mut Probe,
    commands: &mut Commands,
    fixture: &mut Fixture,
    run: &Run,
    assets: (
        &mut Assets<Mesh>,
        &mut Assets<Image>,
        &mut Assets<SurfaceMaterial>,
    ),
) {
    let frame = run.tick - run.pre_roll;
    let mut camera = probe.camera.take().unwrap();
    camera.ending = frame >= fixture.frames.len();
    let ready = probe.bridge.0.lock().unwrap().ready;
    if frame == 0 {
        assert!(
            ready,
            "Protected coarse GPU assets not ready after pre-roll"
        );
        probe.phase = "idle";
        camera.event(probe, frame, "bootstrap");
    }
    let eye = fixture.frames[frame.min(fixture.frames.len() - 1)].eye;
    let plan = probe
        .cpu
        .as_ref()
        .unwrap()
        .plan([eye[0], eye[1]], camera.policy)
        .expect("Camera demand");
    camera.plan = serde_json::json!({"eye":eye,"desired":plan.desired,"loads":plan.loads,"removals":plan.removals,"deferred":plan.deferred});
    let obsolete = camera.ending || !plan.desired.contains(&(probe.region as u64));
    if probe.phase == "loading" && obsolete {
        drop(probe.worker.take());
        probe.phase = "idle";
        camera.cancellations += 1;
        camera.worker_cancellations += 1;
        camera.event(probe, frame, "cancelled_worker");
    } else if probe.phase == "uploading" && obsolete {
        camera.ready_cancellations += u64::from(ready);
        *probe.bridge.0.lock().unwrap() = Watch::owners(&[probe.fine.as_ref().unwrap()], false);
        probe.phase = "retiring";
        probe.changed = frame;
        camera.event(
            probe,
            frame,
            if ready {
                "cancel_ready"
            } else {
                "cancel_pending"
            },
        );
    } else if probe.phase == "retiring" && ready && frame > probe.changed {
        let owners = probe.fine.take().unwrap();
        probe.retiring_bytes = owners.logical_bytes;
        for entity in &owners.entities {
            commands.entity(*entity).despawn();
        }
        for h in &owners.meshes {
            assets.0.remove(h.id());
        }
        for h in &owners.images {
            assets.1.remove(h.id());
        }
        for h in &owners.materials {
            assets.2.remove(h.id());
        }
        *probe.bridge.0.lock().unwrap() = Watch::owners(&[&owners], true);
        drop(owners);
        probe.phase = "unloading";
        camera.event(probe, frame, "unloading");
    } else if probe.phase == "unloading" && ready {
        probe.retiring_bytes = 0;
        let kind = if probe.staged.take().is_some() {
            camera.cancellations += 1;
            "cancelled_gpu"
        } else {
            probe
                .cpu
                .as_mut()
                .unwrap()
                .publish_fine(Vec::new(), &[probe.region as u64])
                .expect("Fine encoded retirement");
            camera.evictions += 1;
            "evicted"
        };
        probe.phase = "idle";
        camera.event(probe, frame, kind);
    } else if probe.phase == "loading" {
        let result = probe
            .worker
            .as_ref()
            .unwrap()
            .receiver
            .lock()
            .unwrap()
            .try_recv();
        match result {
            Ok(result) => {
                let input = result.expect("Camera cooked source failed");
                drop(probe.worker.take());
                let staged = probe.cpu.as_mut().unwrap().stage(
                    &input.manifest,
                    content_id(&input.manifest),
                    std::array::from_fn(|i| input.roots[i].as_slice()),
                    &input.leaves.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                );
                match staged {
                    Err(e) if e.0 == "live bundle byte budget exhausted" => {
                        camera.blocked[probe.region] =
                            Some(probe.cpu.as_ref().unwrap().snapshot().world().epoch());
                        camera.cpu_deferrals += 1;
                        probe.phase = "idle";
                        camera.event(probe, frame, "deferred_cpu");
                    }
                    Err(e) => panic!("Camera fine admission failed: {e}"),
                    Ok(staged) => {
                        let chunk = input.chunk(probe.region).expect("Camera fine conversion");
                        if probe.logical_bytes() + cooked::logical_bytes(&chunk) > probe.budget {
                            drop(staged);
                            camera.blocked[probe.region] =
                                Some(probe.cpu.as_ref().unwrap().snapshot().world().epoch());
                            camera.logical_deferrals += 1;
                            probe.phase = "idle";
                            camera.event(probe, frame, "deferred_logical");
                        } else {
                            let owners = spawn(&chunk, commands, assets.0, assets.1, assets.2);
                            *probe.bridge.0.lock().unwrap() = Watch::owners(&[&owners], false);
                            probe.fine = Some(owners);
                            probe.staged = Some(staged);
                            probe.phase = "uploading";
                            camera.uploads += 1;
                            camera.event(probe, frame, "uploading");
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => panic!("Camera worker disconnected"),
        }
    } else if probe.phase == "uploading" && ready {
        let region = probe.region;
        let staged = probe.staged.take().unwrap();
        probe
            .cpu
            .as_mut()
            .unwrap()
            .publish_fine(vec![(region as u64, staged)], &[])
            .expect("Camera fine publication");
        assert!(probe.active[region].is_none());
        probe.active[region] = probe.fine.take();
        probe.selected[region] = region;
        probe.phase = "idle";
        camera.publications += 1;
        camera.event(probe, frame, "published");
    } else if probe.phase == "idle" {
        let removal = if camera.ending {
            probe.active.iter().position(Option::is_some)
        } else {
            plan.removals.first().map(|&r| r as usize)
        };
        if let Some(region) = removal {
            probe.region = region;
            probe.fine = Some(
                probe.active[region]
                    .take()
                    .expect("Missing committed GPU fine owner"),
            );
            probe.selected[region] = region + 4;
            *probe.bridge.0.lock().unwrap() = Watch::owners(&[probe.fine.as_ref().unwrap()], false);
            probe.phase = "retiring";
            probe.changed = frame;
            camera.event(probe, frame, "retiring");
        } else if !camera.ending {
            let epoch = probe.cpu.as_ref().unwrap().snapshot().world().epoch();
            if let Some(&region) = plan
                .loads
                .iter()
                .find(|&&r| camera.blocked[r as usize] != Some(epoch))
            {
                let region = region as usize;
                let path = probe.paths[region].clone();
                let (sender, receiver) = mpsc::sync_channel(1);
                let join = std::thread::spawn(move || {
                    let result = cooked::Input::load(&path).map_err(|e| e.to_string());
                    let _ = sender.send(result);
                });
                probe.worker = Some(Worker {
                    receiver: Mutex::new(receiver),
                    join: Some(join),
                });
                probe.region = region;
                probe.phase = "loading";
                camera.requests += 1;
                camera.event(probe, frame, "requested");
            }
        }
    }
    // Explicit view choice permits coarse during retirement while encoded fine
    // ownership remains pinned. Never select an unavailable CPU representation.
    let requests = std::array::from_fn::<_, 4, _>(|r| theia_assets::world_coverage::ViewRequest {
        region: r as u64,
        desired: if probe.selected[r] < 4 {
            theia_assets::lod::Level::Fine
        } else {
            theia_assets::lod::Level::Coarse
        },
    });
    let view = probe
        .cpu
        .as_ref()
        .unwrap()
        .snapshot()
        .select_view(&requests)
        .expect("Camera view coverage");
    assert_eq!(
        view.selections()
            .iter()
            .map(|s| s.chunk as usize)
            .collect::<Vec<_>>(),
        probe.selected
    );
    drop(view);
    let index = frame.min(fixture.frames.len() - 1);
    fixture.frames[index].selected = probe.selected;
    if !camera.ending && camera.capture_frames.contains(&frame) {
        probe.capture(commands, run, frame, format!("frame-{frame}"));
    }
    probe.camera = Some(camera);
}
