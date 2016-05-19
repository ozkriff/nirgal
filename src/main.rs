// See LICENSE file for copyright and license details.

#[macro_use]
extern crate glium;

extern crate time;

use std::thread;
use std::time::Duration;
use glium::{glutin, DisplayBuild, Surface};
use glium::index::PrimitiveType;
use glium::glutin::ElementState::{Released};

const FPS: u64 = 60;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

implement_vertex!(Vertex, position, color);

fn make_program(display: &glium::Display) -> glium::Program {
    let api = display.get_window().unwrap().get_api();
    let shader_src_preamble = match api {
        glium::glutin::Api::OpenGl => r"
            #version 120
            #define lowp
            #define mediump
            #define highp
        ",
        glium::glutin::Api::OpenGlEs => r"
            #version 100
        ",
        _ => unimplemented!(),
    }.to_string();
    let vertex_shader_src = shader_src_preamble.clone() + r"
        uniform lowp mat4 matrix;
        attribute lowp vec2 position;
        attribute lowp vec3 color;
        varying lowp vec3 vColor;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0) * matrix;
            vColor = color;
        }
    ";
    let fragment_shader_src = shader_src_preamble + r"
        varying lowp vec3 vColor;
        void main() {
            gl_FragColor = vec4(vColor, 1.0);
        }
    ";
    let program = glium::Program::from_source(
        display,
        &vertex_shader_src,
        &fragment_shader_src,
        None
    ).unwrap();
    program
}

fn create_display() -> glium::Display {
    let gl_version = glutin::GlRequest::GlThenGles {
        opengles_version: (2, 0),
        opengl_version: (2, 1),
    };
    glutin::WindowBuilder::new()
        .with_gl(gl_version)
        .with_depth_buffer(24)
        .with_title("Nergal".to_string())
        .build_glium()
        .unwrap()
}

struct Visualizer {
    display: glium::Display,
    program: glium::Program,
    is_running: bool,
    accumulator: u64,
    previous_clock: u64,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
}

impl Visualizer {
    fn new() -> Visualizer {
        let display = create_display();
        let program = make_program(&display);
        let vertex_buffer = {
            let vertices = [
                Vertex { position: [-0.5, -0.5], color: [0.0, 1.0, 0.0] },
                Vertex { position: [ 0.0,  0.5], color: [0.0, 0.0, 1.0] },
                Vertex { position: [ 0.5, -0.5], color: [1.0, 0.0, 0.0] },
            ];
            glium::VertexBuffer::new(&display, &vertices).unwrap()
        };
        let index_buffer = glium::IndexBuffer::new(
            &display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();
        let accumulator = 0;
        let previous_clock = time::precise_time_ns();
        Visualizer {
            display: display,
            program: program,
            is_running: true,
            accumulator: accumulator,
            previous_clock: previous_clock,
            vertex_buffer: vertex_buffer,
            index_buffer: index_buffer,
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn handle_events(&mut self) {
        let events: Vec<_> = self.display.poll_events().collect();
        for event in events {
            match event {
                // glutin::Event::Resized(x, y) => {}, // TODO
                glutin::Event::Closed => {
                    self.is_running = false;
                },
                glutin::Event::KeyboardInput(Released, _, Some(key)) => {
                    match key {
                        glutin::VirtualKeyCode::Q
                            | glutin::VirtualKeyCode::Escape =>
                        {
                            self.is_running = false;
                        },
                        _ => {},
                    }
                },
                _ => (),
            }
        }
    }

    fn draw(&self) {
        let view_matrix: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let uniforms = uniform! {
            matrix: view_matrix,
        };
        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &uniforms,
            &Default::default(),
        ).unwrap();
        target.finish().unwrap();
    }

    fn update_timer(&mut self) {
        let fixed_time_stamp = 1_000_000_000 / FPS;
        let now = time::precise_time_ns();
        self.accumulator += now - self.previous_clock;
        self.previous_clock = now;
        while self.accumulator >= fixed_time_stamp {
            self.accumulator -= fixed_time_stamp;
            // TODO: update realtime state here
        }
        let remainder_ms = (fixed_time_stamp - self.accumulator) / 1_000_000;
        thread::sleep(Duration::from_millis(remainder_ms));
    }
}

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.draw();
        visualizer.handle_events();
        visualizer.update_timer();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
