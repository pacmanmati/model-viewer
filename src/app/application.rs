use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::render::Renderer;

pub struct Application {
    window: Window,
    frame_rate: f64,
    event_loop: Option<EventLoop<()>>,
    renderer: Renderer,
}

impl Application {
    pub fn new(win_title: &str, frame_rate: f64) -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(win_title)
            .build(&event_loop)
            .unwrap();
        let renderer = Renderer::new(&window);

        Self {
            window,
            frame_rate,
            event_loop: Some(event_loop),
            renderer,
        }
    }

    pub fn run(mut self) {
        let mut last_update_inst = Instant::now();
        let mut last_frame_inst = Instant::now();
        let (mut frame_count, mut accum_time) = (0, 0.0);
        let event_loop = self.event_loop.take().unwrap(); // avoid the self move problem
        self.renderer.init_cube();
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            Event::RedrawEventsCleared => {
                let target_frametime = Duration::from_secs_f64(1.0 / self.frame_rate);
                let time_since_last_frame = last_update_inst.elapsed();
                if time_since_last_frame >= target_frametime {
                    self.window.request_redraw();
                    last_update_inst = Instant::now();
                } else {
                    *control_flow = ControlFlow::WaitUntil(
                        Instant::now() + target_frametime - time_since_last_frame,
                    );
                }
            }
            Event::RedrawRequested(_) => {
                accum_time += last_frame_inst.elapsed().as_secs_f32();
                last_frame_inst = Instant::now();
                frame_count += 1;
                if frame_count == 100 {
                    println!(
                        "avg frame time {}ms.",
                        accum_time * 1000.0 / frame_count as f32
                    );
                    accum_time = 0.0;
                    frame_count = 0;
                }
                Renderer::draw_cube(&self.renderer);
            }
            Event::MainEventsCleared => {}
            _ => {}
        });
    }
}
