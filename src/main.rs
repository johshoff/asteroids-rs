#[macro_use]
extern crate glium;
extern crate clock_ticks;
extern crate nalgebra;
extern crate rustc_serialize;

use std::f32;
use glium::Surface;
use glium::glutin;
use glium::glutin::*;
use glium::index::PrimitiveType;
use nalgebra::Vec2;

mod settings;
use settings::*;

struct Ship {
    pub rotation         : f32,
    pub position         : Vec2<f32>,
    pub velocity         : Vec2<f32>,
    pub prev_position    : Vec2<f32>,
    pub prev_rotation    : f32,
    pub left_is_pressed  : bool,
    pub right_is_pressed : bool,
    pub up_is_pressed    : bool,
}

impl Ship {
    fn new() -> Ship {
        Ship {
            rotation         : 0f32,
            position         : Vec2::new(0.3f32, 0.1f32),
            velocity         : Vec2::new(0.0f32, 0.0f32),
            prev_position    : Vec2::new(0.3f32, 0.1f32),
            prev_rotation    : 0f32,
            left_is_pressed  : false,
            right_is_pressed : false,
            up_is_pressed    : false,
        }
    }
}

fn main() {
    use glium::DisplayBuild;

    let settings = load_settings("settings.json");

    let display = {
        let mut display_builder = glutin::WindowBuilder::new();

        if settings.fullscreen {
            display_builder = display_builder.with_fullscreen(glutin::get_primary_monitor());
        }

        display_builder.build_glium().unwrap()
    };

    let index_buffer = glium::IndexBuffer::new(&display, PrimitiveType::TrianglesList,
                                               &[0, 1, 2,
                                                 3, 4, 5u16]).unwrap();

    let program = program!(&display,
        140 => {
            vertex: "
                #version 140

                uniform mat4 matrix;

                in vec2 position;
                in vec2 global_position;
                in vec3 color;
                in float rotation;

                out vec3 vColor;

                void main() {
                    float cos_rotation = cos(rotation);
                    float sin_rotation = sin(rotation);
                    gl_Position = vec4(
                        position.x * cos_rotation - position.y * sin_rotation + global_position.x,
                        position.x * sin_rotation + position.y * cos_rotation + global_position.y,
                        0.0,
                        1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 140
                in vec3 vColor;
                out vec4 f_color;

                void main() {
                    f_color = vec4(vColor, 1.0);
                }
            "
        },
    ).unwrap();

    let mut accumulator = 0;
    let mut previous_clock = clock_ticks::precise_time_ns();

    let mut frames = 0;
    let mut fpses = Vec::new();

    let mut do_clear = true;

    let uniforms = uniform! {
        matrix: [
            [0.4, 0.0, 0.0, 0.0],
            [0.0, 0.5, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32]
        ]
    };

    let mut ship = Ship::new();

    loop {
        let mut target = display.draw();

        if do_clear {
            target.clear_color(0.0, 0.0, 0.0, 0.0);
        }

        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                position: [f32; 2],
                color: [f32; 3],
                rotation: f32,
                global_position: [f32; 2],
            }

            implement_vertex!(Vertex, position, color, rotation, global_position);

            glium::VertexBuffer::new(&display,
                &[
                    Vertex { position: [-0.05, -0.025], color: [0.3, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() },
                    Vertex { position: [ 0.05,  0.000], color: [0.3, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() },
                    Vertex { position: [-0.05,  0.025], color: [0.3, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() },
                    Vertex { position: [-0.05, -0.025], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() },
                    Vertex { position: [ 0.05,  0.000], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() },
                    Vertex { position: [-0.05,  0.025], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() },
                ]
            ).unwrap()
        };

        target.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &Default::default()).unwrap();

        target.finish().unwrap();

        for event in display.poll_events() {
            match event {
                glutin::Event::Closed => return,
                glutin::Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::C)) => { do_clear = !do_clear },
                glutin::Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Escape)) => return,

                glutin::Event::KeyboardInput(pressed, _, Some(VirtualKeyCode::Left )) => { ship.left_is_pressed  = pressed == ElementState::Pressed },
                glutin::Event::KeyboardInput(pressed, _, Some(VirtualKeyCode::Right)) => { ship.right_is_pressed = pressed == ElementState::Pressed },
                glutin::Event::KeyboardInput(pressed, _, Some(VirtualKeyCode::Up   )) => { ship.up_is_pressed    = pressed == ElementState::Pressed },

                glutin::Event::KeyboardInput(ElementState::Pressed, _, Some(keycode)) => { println!("Key pressed but not handled: {:?}", keycode); },
                _ => ()
            }
        }

        let now = clock_ticks::precise_time_ns();

        if settings.print_fps {
            frames += 1;

            if previous_clock / 1_000_000_000 < now / 1_000_000_000 {
                fpses.push(frames);
                frames = 0;

                if fpses.len() == 5 {
                    for fps in fpses.iter() {
                        println!("FPS {}", fps);
                    }
                    fpses.clear();
                }
            }
        }

        accumulator += now - previous_clock;
        previous_clock = now;

        const FIXED_TIME_STAMP: u64 = 1_000_000; // = 1 millisecond
        while accumulator >= FIXED_TIME_STAMP {
            accumulator -= FIXED_TIME_STAMP;

            ship.prev_position = ship.position;
            ship.prev_rotation = ship.rotation;

            if ship.left_is_pressed {
                ship.rotation += settings.rotation_speed;
            }
            if ship.right_is_pressed {
                ship.rotation -= settings.rotation_speed;
            }
            let acceleration = if ship.up_is_pressed { settings.acceleration } else { 0f32 };
            let direction = Vec2::new(f32::cos(ship.rotation), f32::sin(ship.rotation));
            ship.velocity = (ship.velocity + direction * acceleration) * settings.drag;
            ship.position = ship.position + ship.velocity;
        }
    }

}

