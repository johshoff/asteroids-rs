extern crate clock_ticks;
extern crate nalgebra;
extern crate rustc_serialize;
extern crate capnp;
extern crate mio;

use std::collections::HashMap;
use std::net::SocketAddr;
use nalgebra::Vec2;
use mio::udp::*;
use mio::buf::SliceBuf;
use settings::load_settings;
use common::*;
use network_capnp::{player_status, game_status};
use capnp::serialize_packed;

struct Client {
	last_message       : u64,
	pilot              : Pilot,
}

pub fn run() {
    let server_address = "0.0.0.0:9998".parse().unwrap();
    println!("Listening for clients on {:?}", server_address);
    let socket = UdpSocket::v4().unwrap();
    socket.bind(&server_address).unwrap();

    let settings = load_settings("settings.json");

    let mut accumulator = 0;
    let mut previous_clock = clock_ticks::precise_time_ns();
    let mut prev_message_sent = previous_clock;
    let reader_options = ::capnp::message::ReaderOptions::new();

    let mut clients : HashMap<SocketAddr, Client> = HashMap::new();

    loop {
        let now = clock_ticks::precise_time_ns();
        accumulator += now - previous_clock;
        previous_clock = now;

        let mut buffer = Vec::new(); // TODO: reuse buffer
        let result = socket.recv_from(&mut buffer);
        if let Ok(Some(from_address)) = result {
            let message_reader = ::capnp::serialize_packed::read_message(&mut
                ::std::io::BufReader::new(
                    ::std::io::Cursor::new(buffer)),
                reader_options).unwrap();

            let message = message_reader.get_root::<player_status::Reader>().unwrap();

            let updated = {
                match clients.get_mut(&from_address) {
                    Some(ref mut client) => {
                        client.pilot.left_is_pressed  = message.get_turn_left();
                        client.pilot.right_is_pressed = message.get_turn_right();
                        client.pilot.up_is_pressed    = message.get_throttle();
                        client.last_message           = now;
                        true
                    }
                    None => false
                }
            };

            if !updated {
                println!("New client from {:?}", from_address);
                let mut pilot = Pilot::new(Integrator::ForwardEuler);
                pilot.spawn().ok();

                clients.insert(from_address, Client { last_message: now, pilot: pilot });
            }
        }

        const FIXED_TIME_STAMP: u64 = 1_000_000; // = 1 millisecond
        while accumulator >= FIXED_TIME_STAMP {
            accumulator -= FIXED_TIME_STAMP;

            for (_, client) in clients.iter_mut() {
				let player = &mut client.pilot;
                match player.ship {
                    None => {}
                    Some(ref mut ship) => {
                        let prev_prev = ship.prev_position;

                        ship.prev_position = ship.position;
                        ship.prev_rotation = ship.rotation;

                        if player.left_is_pressed {
                            ship.rotation += settings.rotation_speed;
                        }
                        if player.right_is_pressed {
                            ship.rotation -= settings.rotation_speed;
                        }
                        let acceleration = if player.up_is_pressed { settings.acceleration } else { 0f32 };
                        let direction = Vec2::new(f32::cos(ship.rotation), f32::sin(ship.rotation));

                        match player.integrator {
                            Integrator::ForwardEuler => {
                                ship.velocity = (ship.velocity + direction * acceleration) * (1f32 - settings.drag);
                                ship.position = ship.position + ship.velocity;
                            },
                            Integrator::Verlet => {
                                let instantaneous_velocity = ship.position - prev_prev;
                                let drag = instantaneous_velocity * settings.drag;
                                ship.position = ship.position + ship.position - prev_prev + (direction * acceleration) - drag;
                            },
                        }
                    }
                }
            }
        }

        if now - prev_message_sent >= settings.message_interval_ms * 1_000_000 {
			// prune timed out players
			{
			    let timedout_clients : Vec<SocketAddr> = clients.iter()
			        .filter(|&(_key, client)| (now - client.last_message) / 1_000_000 > settings.client_timeout_ms)
			        .map(|(key, _)| key.clone())
			        .collect();
			    for client in timedout_clients {
			        println!("Timed out client {:?}", client);
				    clients.remove(&client);
			    }
			}

            let game_status_msg = {
                let mut message = ::capnp::message::Builder::new_default();
                {
                    let mut p = message.init_root::<game_status::Builder>();
                    p.set_timestamp(now);

                    let num_ships = clients.values().filter(|client| client.pilot.ship().is_some()).count();
                    let mut ships = p.borrow().init_ships(num_ships as u32);
                    let mut count = 0;

                    for client in clients.values()
                    {
                        if let Some(ref ship) = client.pilot.ship {
                            let mut ship_msg = ships.borrow().get(count);
                            let velocity = ship.position - ship.prev_position;
                            ship_msg.set_id(0);
                            ship_msg.set_x(ship.position.x);
                            ship_msg.set_y(ship.position.y);
                            ship_msg.set_dx(velocity.x);
                            ship_msg.set_dy(velocity.y);
                            ship_msg.set_ang(ship.rotation);
                            ship_msg.set_dang(ship.rotation - ship.prev_rotation);

                            count += 1;
                        }
                    }
                }

                message
            };

            for address in clients.keys() {
                let mut buffer = Vec::new();
                serialize_packed::write_message(&mut buffer, &game_status_msg).unwrap();
                let result = socket.send_to(&mut SliceBuf::wrap(&buffer), &address);
                result.unwrap();
            }
            prev_message_sent = now;
        }
    }
}

