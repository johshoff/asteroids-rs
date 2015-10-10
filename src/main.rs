#[macro_use]
extern crate glium;
extern crate clock_ticks;
extern crate nalgebra;
extern crate rustc_serialize;
extern crate capnp;
extern crate mio;

mod network_capnp;
mod settings;
mod server;
mod common;

use glium::Surface;
use glium::glutin;
use glium::glutin::*;
use glium::index::PrimitiveType;
use nalgebra::Vec2;
use mio::udp::*;
use mio::buf::SliceBuf;
use capnp::serialize_packed;

use network_capnp::{player_status, game_status};
use settings::*;
use common::*;

struct LocalPlayer {
    pub pilot            : Pilot,
    pub left_key         : VirtualKeyCode,
    pub right_key        : VirtualKeyCode,
    pub up_key           : VirtualKeyCode,
}

impl LocalPlayer {
    fn new(left_key : VirtualKeyCode, right_key : VirtualKeyCode, up_key : VirtualKeyCode, integrator : Integrator) -> Self {
        LocalPlayer {
            pilot            : Pilot::new(integrator),
            left_key         : left_key,
            right_key        : right_key,
            up_key           : up_key,
        }
    }

    fn spawn(&mut self) -> Result<(), ()> {
        self.pilot.spawn()
    }

    fn on_key(&mut self, key: VirtualKeyCode, pressed: bool) -> bool {
        if      key == self.left_key  { self.pilot.left_is_pressed  = pressed; true }
        else if key == self.right_key { self.pilot.right_is_pressed = pressed; true }
        else if key == self.up_key    { self.pilot.up_is_pressed    = pressed; true }
        else { false }
    }
}

fn main() {
    for argument in ::std::env::args().skip(1) {
        match argument.as_ref() {
            "server" => { ::server::run(); return; },
            unknown  => panic!(format!("Unknown argument '{}'", unknown)),
        }
    }
    client();
}

fn client() {
    use glium::DisplayBuild;

    let server_address = "0.0.0.0:9998".parse().unwrap();
    let socket = UdpSocket::v4().unwrap();

    let settings = load_settings("settings.json");

    let display = {
        let mut display_builder = glutin::WindowBuilder::new();

        if settings.fullscreen {
            display_builder = display_builder.with_fullscreen(glutin::get_primary_monitor());
        }

        display_builder.build_glium().unwrap()
    };

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

    let mut previous_clock = clock_ticks::precise_time_ns();
    let mut prev_message_sent = previous_clock;

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

    let mut players = Vec::new();

    players.push(LocalPlayer::new(VirtualKeyCode::Left,
                                  VirtualKeyCode::Right,
                                  VirtualKeyCode::Up,
                                  Integrator::ForwardEuler));
/*
    players.push(LocalPlayer::new(VirtualKeyCode::A,
                                  VirtualKeyCode::D,
                                  VirtualKeyCode::W,
                                  Integrator::Verlet));

    players.push(LocalPlayer::new(VirtualKeyCode::F,
                                  VirtualKeyCode::H,
                                  VirtualKeyCode::T,
                                  Integrator::ForwardEuler));
*/
    for player in players.iter_mut() {
        player.spawn().unwrap();
    }

    let mut buffer = Vec::new();
    let mut remote_ships : Vec<Ship> = Vec::new();
    let mut last_message_timestamp = 0;

    loop {
        let mut target = display.draw();

        if do_clear {
            target.clear_color(0.0, 0.0, 0.0, 0.0);
        }

        let (vertex_buffer, index_buffer) = {
            #[derive(Copy, Clone)]
            struct Vertex {
                position: [f32; 2],
                color: [f32; 3],
                rotation: f32,
                global_position: [f32; 2],
            }

            implement_vertex!(Vertex, position, color, rotation, global_position);

            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            /*
            for player in players.iter() {
                match *player.ship() {
                    None => {}
                    Some(ref ship) => {
                        let base_index = vertices.len() as u16;
                        for i in 0..6 {
                            indices.push(base_index + i);
                        }

                        vertices.push(Vertex { position: [-0.05, -0.025], color: [1.0, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() });
                        vertices.push(Vertex { position: [ 0.05,  0.000], color: [1.0, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() });
                        vertices.push(Vertex { position: [-0.05,  0.025], color: [1.0, 0.3, 0.3], rotation: ship.prev_rotation, global_position: *ship.prev_position.as_array() });
                        vertices.push(Vertex { position: [-0.05, -0.025], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() });
                        vertices.push(Vertex { position: [ 0.05,  0.000], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() });
                        vertices.push(Vertex { position: [-0.05,  0.025], color: [1.0, 1.0, 1.0], rotation: ship.rotation     , global_position: *ship.position.as_array() });
                    }
                };
            }
            */

            let since_message = (clock_ticks::precise_time_ns() - last_message_timestamp) as f32 / 1_000_000f32;
            for ship in remote_ships.iter() {
                let base_index = vertices.len() as u16;
                for i in 0..3 {
                    indices.push(base_index + i);
                }

                // dead reconning position
                let position = ship.position + ship.velocity * since_message;
                let rotation = ship.rotation + ship.rotational_velocity * since_message;

                vertices.push(Vertex { position: [-0.05, -0.025], color: [1.0, 1.0, 1.0], rotation: rotation, global_position: *position.as_array() });
                vertices.push(Vertex { position: [ 0.05,  0.000], color: [1.0, 1.0, 1.0], rotation: rotation, global_position: *position.as_array() });
                vertices.push(Vertex { position: [-0.05,  0.025], color: [1.0, 1.0, 1.0], rotation: rotation, global_position: *position.as_array() });
            }

            let vertex_buffer = glium::VertexBuffer::new(&display, &vertices).unwrap();
            let index_buffer = glium::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &indices).unwrap();

            (vertex_buffer, index_buffer)
        };

        target.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &Default::default()).unwrap();

        target.finish().unwrap();

        for event in display.poll_events() {
            match event {
                glutin::Event::Closed => return,
                glutin::Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::C)) => { do_clear = !do_clear },
                glutin::Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Escape)) => return,

                glutin::Event::KeyboardInput(pressed, _, Some(key)) => {
                    let is_pressed = pressed == ElementState::Pressed;
                    let handled = players.iter_mut().any(|player| player.on_key(key, is_pressed));

                    if !handled && is_pressed {
                        println!("Key pressed but not handled: {:?}", key);
                    }
                },

                _ => ()
            }
        }

        let now = clock_ticks::precise_time_ns();

        if settings.print_fps {
            frames += 1;

            if previous_clock / 1_000_000_000 < now / 1_000_000_000 {
                fpses.push(frames);
                frames = 0;

                if fpses.len() == 1 {
                    for fps in fpses.iter() {
                        println!("FPS {}, last_message_timestamp={}", fps, last_message_timestamp);
                    }
                    fpses.clear();
                }
            }
        }

        if now - prev_message_sent >= settings.message_interval_ms * 1_000_000 {
            for player in players.iter() {
                let mut message = ::capnp::message::Builder::new_default();
                {
                    let mut p = message.init_root::<player_status::Builder>();
                    p.set_throttle  (player.pilot.up_is_pressed);
                    p.set_turn_left (player.pilot.left_is_pressed);
                    p.set_turn_right(player.pilot.right_is_pressed);
                }

                serialize_packed::write_message(&mut buffer, &message).unwrap();

                let result = socket.send_to(&mut SliceBuf::wrap(&buffer), &server_address);

                result.unwrap();

                buffer.clear();
            }
            prev_message_sent = now;
        }

        {
            let reader_options = ::capnp::message::ReaderOptions::new();
            let mut buffer = Vec::new(); // TODO: reuse buffer
            let result = socket.recv_from(&mut buffer);
            if let Ok(Some(_from_address)) = result {
                let message_reader = ::capnp::serialize_packed::read_message(&mut
                    ::std::io::BufReader::new(
                        ::std::io::Cursor::new(buffer)),
                    reader_options).unwrap();

                let message = message_reader.get_root::<game_status::Reader>().unwrap();

                last_message_timestamp = now;
                remote_ships.clear();

                for ship_msg in message.get_ships().unwrap().iter() {
                    let mut ship = Ship::new();
                    ship.rotation = ship_msg.get_ang();
                    ship.rotational_velocity = ship_msg.get_dang();
                    ship.position = Vec2::new(ship_msg.get_x() , ship_msg.get_y());
                    ship.velocity = Vec2::new(ship_msg.get_dx(), ship_msg.get_dy());

                    remote_ships.push(ship);
                }
            }
        }

        previous_clock = now;
    }

}

