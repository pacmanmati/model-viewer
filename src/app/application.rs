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
                // render here
                let frame = match self.renderer.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        self.renderer
                            .surface
                            .configure(&self.renderer.device, &self.renderer.surface_config);
                        self.renderer
                            .surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture.")
                    }
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = self
                    .renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.1,
                                    b: 0.6,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                    rpass.push_debug_group("preparing data for drawing...");
                    rpass.set_pipeline(self.renderer.active_pipeline.as_ref().unwrap());
                    rpass.set_bind_group(0, self.renderer.bind_group.as_ref().unwrap(), &[]);
                    rpass.set_index_buffer(
                        self.renderer.index_buffer.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    rpass.set_vertex_buffer(
                        0,
                        self.renderer.vertex_buffer.as_ref().unwrap().slice(..),
                    );
                    rpass.pop_debug_group();
                    rpass.insert_debug_marker("drawing");
                    rpass.draw_indexed(0..self.renderer.index_count as u32, 0, 0..1);
                }
                self.renderer.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::MainEventsCleared => {}
            _ => {}
        });
    }
}
