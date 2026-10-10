mod gpu;
mod pipeline;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Default)]
struct Options {
    no_validation: bool,
    gpu: Option<String>,
    frames: Option<u64>,
    smoke: bool,
    clear_only: bool,
    shader_dir: Option<std::path::PathBuf>,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut result = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--no-validation" => result.no_validation = true,
                "--gpu" => result.gpu = Some(args.next().ok_or("--gpu needs a name substring")?),
                "--frames" => {
                    let count = args
                        .next()
                        .ok_or("--frames needs a positive integer")?
                        .parse()?;
                    if count == 0 {
                        return Err("--frames must be positive".into());
                    }
                    result.frames = Some(count);
                }
                "--clear-only" => result.clear_only = true,
                "--shader-dir" => {
                    result.shader_dir =
                        Some(args.next().ok_or("--shader-dir needs a directory")?.into())
                }
                "--smoke-test" => result.smoke = true,
                "--help" => {
                    println!(
                        "Theia Render Lab: Slang mesh triangle qualification\n  --gpu <name substring>\n  --frames <positive count>\n  --smoke-test  120 frames, resize/minimize/restore with validation\n  --shader-dir <directory>  compiled SPIR-V directory\n  --clear-only  run the qualified clear-screen path without shaders\n  --no-validation  explicitly disable validation"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}").into()),
            }
        }
        if result.smoke {
            if result.no_validation || result.frames.is_some() {
                return Err(
                    "--smoke-test requires validation and cannot combine with --frames".into(),
                );
            }
            result.frames = Some(120);
        }
        Ok(result)
    }
}
struct App {
    options: Options,
    validation: Arc<gpu::Validation>,
    window: Option<Arc<Window>>,
    renderer: Option<gpu::Renderer>,
    frames: u64,
    restore: Option<Instant>,
    failure: Option<String>,
    occluded: bool,
}
impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl ToString) {
        self.failure = Some(error.to_string());
        event_loop.exit();
    }
    fn shutdown(&mut self) {
        self.renderer.take();
        self.window.take();
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<_> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Theia Render Lab — M0-D mesh triangle")
                        .with_inner_size(PhysicalSize::new(960, 540)),
                )?,
            );
            let renderer =
                gpu::Renderer::new(window.clone(), &self.options, self.validation.clone())?;
            Ok((window, renderer))
        })();
        match result {
            Ok((window, renderer)) => {
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.shutdown();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                event_loop.exit()
            }
            WindowEvent::Resized(size) => {
                println!("Window resized: {}x{}", size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.recreate = true;
                }
            }
            WindowEvent::Occluded(value) => self.occluded = value,
            WindowEvent::RedrawRequested => {
                if self.restore.is_some() || self.occluded {
                    return;
                }
                let size = self.window.as_ref().unwrap().inner_size();
                let result = self.renderer.as_mut().unwrap().draw(size);
                match result {
                    Ok(true) => self.frames += 1,
                    Ok(false) => return,
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                }
                if self.options.smoke {
                    let window = self.window.as_ref().unwrap();
                    match self.frames {
                        20 => {
                            let _ = window.request_inner_size(PhysicalSize::new(640, 360));
                        }
                        40 => {
                            let _ = window.request_inner_size(PhysicalSize::new(1280, 720));
                        }
                        60 => {
                            // Exercise zero-size handling explicitly: X11 need not report 0x0 on minimize.
                            match self
                                .renderer
                                .as_mut()
                                .unwrap()
                                .draw(PhysicalSize::new(0, 0))
                            {
                                Ok(false) => println!("Smoke: zero-size draw deferred"),
                                _ => {
                                    self.fail(event_loop, "Zero-size draw was not deferred");
                                    return;
                                }
                            }
                            window.set_minimized(true);
                            self.restore = Some(Instant::now() + Duration::from_millis(300));
                            println!("Smoke: requested minimize");
                        }
                        _ => {}
                    }
                }
                if self
                    .options
                    .frames
                    .is_some_and(|limit| self.frames >= limit)
                {
                    println!("Presented {} frames", self.frames);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.restore {
            if Instant::now() < deadline {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                return;
            }
            self.restore = None;
            self.occluded = false;
            if let Some(window) = &self.window {
                window.set_minimized(false);
                let _ = window.request_inner_size(PhysicalSize::new(800, 600));
                println!("Smoke: requested restore");
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.recreate = true;
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
        if !self.occluded
            && let Some(window) = &self.window
        {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                window.request_redraw();
            }
        }
    }
    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.shutdown();
    }
}
fn main() -> Result<()> {
    let options = Options::parse()?;
    let validation = Arc::new(gpu::Validation::default());
    let mut app = App {
        options,
        validation: validation.clone(),
        window: None,
        renderer: None,
        frames: 0,
        restore: None,
        failure: None,
        occluded: false,
    };
    let result = EventLoop::new()?.run_app(&mut app);
    app.shutdown();
    let (warnings, errors) = validation.counts();
    println!("Validation totals (including teardown): {warnings} warnings, {errors} errors");
    result?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if errors > 0 {
        return Err("Vulkan validation reported errors".into());
    }
    Ok(())
}
