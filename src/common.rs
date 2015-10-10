extern crate nalgebra;

use nalgebra::Vec2;

pub enum Integrator {
    ForwardEuler,
    Verlet,
}

pub struct Ship {
    pub rotation            : f32,
    pub rotational_velocity : f32,
    pub position            : Vec2<f32>,
    pub velocity            : Vec2<f32>,
    pub prev_position       : Vec2<f32>,
    pub prev_rotation       : f32,
}

pub struct Pilot {
    pub ship             : Option<Ship>,
    pub left_is_pressed  : bool,
    pub right_is_pressed : bool,
    pub up_is_pressed    : bool,
    pub integrator       : Integrator,
}

impl Pilot {
    pub fn new(integrator : Integrator) -> Self {
        Pilot {
            ship             : None,
            left_is_pressed  : false,
            right_is_pressed : false,
            up_is_pressed    : false,
            integrator       : integrator,
        }
    }

    pub fn spawn(&mut self) -> Result<(), ()> {
        match self.ship {
            None => { self.ship = Some(Ship::new()); Ok(()) },
            _    => Err(()),
        }
    }

    pub fn ship(&self) -> &Option<Ship> {
        &self.ship
    }
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            rotation            : 0f32,
            rotational_velocity : 0f32,
            position            : Vec2::new(0.3f32, 0.1f32),
            velocity            : Vec2::new(0.0f32, 0.0f32),
            prev_position       : Vec2::new(0.3f32, 0.1f32),
            prev_rotation       : 0f32,
        }
    }
}

