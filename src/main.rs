// See LICENSE file for copyright and license details.

#[macro_use]
extern crate glium;

extern crate time;
extern crate image;
extern crate cgmath;
extern crate rand;

#[cfg(target_os = "android")]
extern crate android_glue;

mod fs;
mod md5;

use std::path::{Path};
use std::f32::consts::{PI};
use std::thread;
use std::time::Duration;
use rand::{thread_rng, Rng};
use glium::{glutin, Texture2d, DisplayBuild, Surface};
use glium::index::PrimitiveType;
use glium::glutin::ElementState::{Pressed, Released};
use cgmath::{Matrix4, Matrix3, Vector3, Vector2, Rad};

const FPS: u64 = 60;

#[derive(Debug, Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

fn load_texture<P: AsRef<Path>>(display: &glium::Display, path: P) -> Texture2d {
    let f = fs::load(path);
    let image = image::load(f, image::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba(
        image.into_raw(), image_dimensions);
    Texture2d::new(display, image).unwrap()
}

fn make_program(display: &glium::Display) -> glium::Program {
    let api = display.get_window().unwrap().get_api();
    let pre_src = fs::load_string(match api {
        glutin::Api::OpenGl => "pre_gl.glsl",
        glutin::Api::OpenGlEs => "pre_gles.glsl",
        _ => unimplemented!(),
    });
    let vs_src = pre_src.clone() + &fs::load_string("vs.glsl");
    let fs_src = pre_src + &fs::load_string("fs.glsl");
    glium::Program::from_source(display, &vs_src, &fs_src, None).unwrap()
}

fn view_matrix(angle_x: Rad<f32>, angle_y: Rad<f32>, zoom: f32, aspect: f32) -> Matrix4<f32> {
    let perspective_mat = cgmath::perspective(Rad::new(PI / 4.0), aspect, 0.1, 100.0);
    let tr_mat = Matrix4::from_translation(Vector3{x: 0.0, y: 0.0, z: -zoom});
    let angle_x_m = Matrix4::from(Matrix3::from_angle_z(angle_x));
    let angle_y_m = Matrix4::from(Matrix3::from_angle_x(angle_y));
    perspective_mat * tr_mat * angle_y_m * angle_x_m
}

fn draw_parameters() -> glium::DrawParameters<'static> {
    glium::DrawParameters {
        depth: glium::Depth {
            test: glium::draw_parameters::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn win_size(display: &glium::Display) -> (u32, u32) {
    let window = display.get_window().unwrap();
    window.get_inner_size().unwrap()
}

fn aspect(display: &glium::Display) -> f32 {
    let (x, y) = win_size(display);
    x as f32 / y as f32
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

// TODO: это все надо как-то более человечно сделать и названия нормальные дать
struct MeshRenderInfo {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    texture: Texture2d,
}

fn prepare_mesh(mesh: &md5::Mesh, display: &glium::Display) -> MeshRenderInfo {
    let prim_type = PrimitiveType::TrianglesList;
    let index_buffer = glium::IndexBuffer::new(
        display, prim_type, mesh.indices()).unwrap();
    let vertex_buffer = glium::VertexBuffer::new(
        display, mesh.vertices()).unwrap();
    let texture = load_texture(display, mesh.shader());
    MeshRenderInfo {
        vertex_buffer: vertex_buffer,
        index_buffer: index_buffer,
        texture: texture,
    }
}

fn prepare_model(model: &md5::Model, display: &glium::Display) -> Vec<MeshRenderInfo> {
    let mut v = Vec::new();
    for mesh in model.meshes() {
        v.push(prepare_mesh(mesh, display));
    }
    v
}

struct A {
    frame: usize,
    anim: md5::Anim,
}

struct Visualizer {
    display: glium::Display,
    program: glium::Program,
    is_running: bool,
    accumulator: u64,
    previous_clock: u64,
    camera_angle_x: Rad<f32>,
    camera_angle_y: Rad<f32>,
    zoom: f32,
    aspect: f32,
    mouse_pos: Vector2<i32>,
    is_lmb_pressed: bool,
    model: md5::Model,
    model_render_infos: Vec<MeshRenderInfo>,
    a: Vec<A>,
}

impl Visualizer {
    fn new() -> Visualizer {
        let display = create_display();
        let program = make_program(&display);
        let aspect = aspect(&display);
        let model = md5::load_model("simpleMan2.6.md5mesh");
        let model_render_infos = prepare_model(&model, &display);
        let mut a = Vec::new();
        for _ in 0..16 {
            let anim = md5::load_anim("simpleMan2.6.md5anim");
            a.push(A {
                frame: thread_rng().gen_range(0, anim.len()),
                anim: anim,
            });
        }
        let accumulator = 0;
        let previous_clock = time::precise_time_ns();
        Visualizer {
            display: display,
            program: program,
            is_running: true,
            accumulator: accumulator,
            previous_clock: previous_clock,
            camera_angle_x: Rad::new(-PI / 6.0),
            camera_angle_y: Rad::new(-PI / 2.0 + PI / 6.0),
            zoom: 5.0,
            aspect: aspect,
            mouse_pos: Vector2{x: 0, y: 0},
            is_lmb_pressed: false,
            model: model,
            model_render_infos: model_render_infos,
            a: a,
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn handle_events(&mut self) {
        self.aspect = aspect(&self.display);
        let rotate_step = PI / 12.0;
        let events: Vec<_> = self.display.poll_events().collect();
        for event in events {
            match event {
                glutin::Event::Resized(x, y) => {
                    self.aspect = x as f32 / y as f32;
                },
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
                        glutin::VirtualKeyCode::Right => self.camera_angle_x.s += rotate_step,
                        glutin::VirtualKeyCode::Left => self.camera_angle_x.s -= rotate_step,
                        glutin::VirtualKeyCode::Down => self.camera_angle_y.s += rotate_step,
                        glutin::VirtualKeyCode::Up => self.camera_angle_y.s -= rotate_step,
                        glutin::VirtualKeyCode::Equals => self.zoom *= 0.75,
                        glutin::VirtualKeyCode::Subtract => self.zoom *= 1.25,
                        _ => {},
                    }
                },
                glutin::Event::MouseMoved(x, y) => {
                    self.handle_mouse_move(Vector2{x: x, y: y});
                },
                glutin::Event::MouseInput(Pressed, glutin::MouseButton::Left) => {
                    self.is_lmb_pressed = true;
                },
                glutin::Event::MouseInput(Released, glutin::MouseButton::Left) => {
                    self.is_lmb_pressed = false;
                },
                glutin::Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                    let pos = Vector2{x: x as i32, y: y as i32};
                    match phase {
                        glutin::TouchPhase::Moved => {
                            self.handle_mouse_move(pos);
                        },
                        glutin::TouchPhase::Started => {
                            self.is_lmb_pressed = true;
                            self.mouse_pos = pos;
                        },
                        glutin::TouchPhase::Ended => {
                            self.mouse_pos = pos;
                            self.is_lmb_pressed = false;
                        },
                        glutin::TouchPhase::Cancelled => unimplemented!(),
                    }
                },
                _ => (),
            }
        }
    }

    fn handle_mouse_move(&mut self, pos: Vector2<i32>) {
        let diff = self.mouse_pos - pos;
        self.mouse_pos = pos;
        if self.is_lmb_pressed {
            let (w, h) = win_size(&self.display);
            self.camera_angle_x.s += PI * (diff.x as f32 / w as f32);
            self.camera_angle_y.s += PI * (diff.y as f32 / h as f32);
        }
    }

    fn draw_model_at(&self, target: &mut glium::Frame, model_mat: [[f32; 4]; 4]) {
        let view_mat: [[f32; 4]; 4] = view_matrix(
            self.camera_angle_x,
            self.camera_angle_y,
            self.zoom,
            self.aspect,
        ).into();
        for ri in &self.model_render_infos {
            let uniforms = uniform! {
                view_mat: view_mat,
                model_mat: model_mat,
                tex: &ri.texture,
            };
            target.draw(
                &ri.vertex_buffer,
                &ri.index_buffer,
                &self.program,
                &uniforms,
                &draw_parameters(),
            ).unwrap();
        }
    }

    fn draw(&mut self) {
        let mut target = self.display.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
        for x in 0..4 {
            for y in 0..4 {
                {
                    let a = &mut self.a[y * 4 + x];
                    self.model.compute(a.anim.joints());
                    for (i, ri) in self.model_render_infos.iter_mut().enumerate() {
                        let vertices = self.model.meshes()[i].vertices();
                        ri.vertex_buffer = glium::VertexBuffer::new(
                            &self.display, vertices).unwrap();
                    }
                }
                let t = Vector3{x: x as f32 * 2.0, y: y as f32 * 2.0, z: 0.0};
                let m = Matrix4::from_translation(t).into();
                self.draw_model_at(&mut target, m);
            }
        }
        target.finish().unwrap();
    }

    fn update_anim(&mut self) {
        for a in &mut self.a {
            a.anim.set_frame(a.frame);
            a.frame += 1;
            if a.frame == a.anim.len() {
                a.frame = 0;
            }
        }
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
        visualizer.update_anim();
        visualizer.handle_events();
        visualizer.update_timer();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
