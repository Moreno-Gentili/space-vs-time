use spacetimedb::SpacetimeType;

#[derive(SpacetimeType)]
#[derive(Copy, Clone)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl PartialEq for Vector3D {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Vector3D {
    pub const ZERO: Velocity = Velocity {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn delta_norm(&self, other: &Vector3D) -> f32 {
        let x = other.x - self.x;
        let y = other.y - self.y;
        let z = other.z - self.z;

        (Vector3D { x, y, z }).norm()
    }

    pub fn norm(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn try_normalize(&self, min_norm: f32) -> Option<Vector3D> {
        let n = self.norm();
        if n < min_norm {
            None
        } else {
            Some(Vector3D {
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            })
        }
    }
}

pub type Position = Vector3D;
pub type Velocity = Vector3D;
pub type Dimension = Vector3D;

impl Position {
    pub fn get_velocity_vector(&self, destination: &Position, speed: f32, tolerance: f32) -> Velocity {
        let dx = destination.x - self.x;
        let dy = destination.y - self.y;
        let dz = destination.z - self.z;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        if distance <= tolerance {
            return Velocity::ZERO;
        }

        Velocity {
            x: dx / distance * speed,
            y: dy / distance * speed,
            z: dz / distance * speed,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
#[derive(SpacetimeType)]
pub enum EntityKind {
    SpacePlayer,
    TimePlayer,
    Ball
}

#[derive(PartialEq, Eq, Clone, Copy)]
#[derive(SpacetimeType)]
pub enum GameState {
    Waiting,
    Reset,
    GameStarted,
    SpaceScored,
    TimeScored,
    GameEndedInSpaceWin,
    GameEndedInTimeWin,
    GameEndedInTie
}


#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone)]
pub enum EntityState {
    Spawning,
    ReadyToMove,
    Moving,
    ReadyToThrow,
    Throwing,
    Hit,
    LostGame,
    WonGame,
    Standby,
    Despawning
}

#[derive(PartialEq, Eq, Clone)]
#[derive(SpacetimeType)]
pub enum RouteAction {
    Move,
    Throw,
    Hit
}


#[derive(PartialEq, Clone)]
pub enum Side {
    Space,
    Time
}