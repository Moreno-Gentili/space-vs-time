use rapier3d::crossbeam::channel::unbounded;
use rapier3d::crossbeam::channel::Receiver;
use rapier3d::prelude::*;
use crate::ecs::components::*;
use crate::ecs::entities::*;
use crate::DEAD_ZONE;
use crate::FIELD_DIMENSION;
use crate::PLAYER_DIMENSION;
use std::collections::HashMap;

const BOUNDARY: Group = Group::GROUP_1;
const SPACE_WALL: Group = Group::GROUP_2;
const TIME_WALL: Group = Group::GROUP_3;
const SPACE_PLAYER: Group = Group::GROUP_4;
const TIME_PLAYER: Group = Group::GROUP_5;
const BALL: Group = Group::GROUP_6;

const GRAVITY: f32 = 9.81;

pub struct PhysicsEngine {
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    physics_hooks: (),
    event_handler: ChannelEventCollector,
    references: HashMap<u128, Reference>,
    collision_receiver: Receiver<CollisionEvent>,
    player_wall_collisions: Vec<PlayerWallCollision>,
    player_ball_collisions: Vec<PlayerBallCollision>,
    ball_wall_collisions: Vec<BallWallCollision>
}

// TODO: should this hold references?
struct Reference {
    rigid_body_handle: RigidBodyHandle,
    collider_handle: ColliderHandle,
    name: String,
    partner_name: Option<String>,
    kind: EntityKind,
    half_z: f32,
    joint: Option<ImpulseJointHandle>
}

#[derive(Clone)]
pub struct PlayerWallCollision {
    pub player_name: String
}

#[derive(Clone)]
pub struct BallWallCollision {
    pub ball_name: String
}

#[derive(Clone)]
pub struct PlayerBallCollision {
    pub player_name: String,
    pub ball_name: String,
    pub tossed_by_name: Option<String>,
    pub velocity: Velocity,
    pub ball_has_rebounded: bool
}

enum CollisionType {
    PlayerBall(PlayerBallCollision),
    PlayerWall(PlayerWallCollision),
    BallWall(BallWallCollision),
    Ignore
}

enum ReferenceResult<'a> {
    PlayerReference(&'a Reference),
    BallReference(&'a Reference),
    Wall,
    Floor
}

enum BallCollisionGroup {
    Full,
    OnlySpace,
    OnlyTime,
    Empty
}

impl PhysicsEngine {
    pub fn new(dimension: Dimension) -> Self {
        let (collision_sender, collision_receiver) = unbounded();
        let (contact_force_sender, _) = unbounded();

        let mut instance: PhysicsEngine = Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            // integration_parameters: IntegrationParameters { contact_damping_ratio: 300.0, ..IntegrationParameters::default() },
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            physics_hooks: (),
            event_handler: ChannelEventCollector::new(collision_sender, contact_force_sender),
            references: HashMap::new(),
            collision_receiver,
            player_ball_collisions: vec![],
            player_wall_collisions: vec![],
            ball_wall_collisions: vec![]
        };

        instance.add_boundaries(dimension);

        instance
    }

    pub fn upsert_ball(&mut self, movable: &Movable, radius: f32, restitution: f32) {
        let identifier: u128 = get_identifier(&movable.name);
        if let Err(_) = self.delete(&movable.name) {
            // Ignore, it's an upsert
        }

        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![movable.position.x, movable.position.y, movable.position.z + radius])
            .linvel(vector![movable.velocity.x, movable.velocity.y, movable.velocity.z])
            .angular_damping(3.0)
            .linear_damping(0.0)
            .can_sleep(false)   // prevents it from ever being allowed to sleep
            .sleeping(false)
            .user_data(identifier)
            .build();

        let collider = ColliderBuilder::ball(radius)
            .restitution(restitution)
            .collision_groups(InteractionGroups { memberships: BALL, filter: BOUNDARY | BALL | SPACE_PLAYER | TIME_PLAYER })
            .friction(2.0)
            .friction_combine_rule(CoefficientCombineRule::Max)
            .user_data(identifier)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .mass(1.0)
            .build();

        let rigid_body_handle = self.rigid_body_set.insert(rigid_body);
        let collider_handle = self.collider_set.insert_with_parent(collider, rigid_body_handle, &mut self.rigid_body_set);

        self.references.insert(identifier, create_reference(rigid_body_handle, collider_handle, radius, movable));
    }

    pub fn upsert_player(&mut self, movable: &Movable, dimension: Dimension, restitution: f32) {
        let identifier: u128 = get_identifier(&movable.name);
        if let Err(_) = self.delete(&movable.name) {
            // Ignore, it's an upsert
        }

        let mut rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![movable.position.x, movable.position.y, movable.position.z + dimension.z / 2.0])
            .can_sleep(false)   // prevents it from ever being allowed to sleep
            .sleeping(false) 
            .linear_damping(10.0)
            .angular_damping(10.0)
            .user_data(identifier)
            .build();

        // rigid_body.set_enabled_translations(true, true, false, true);
        rigid_body.set_enabled_translations(true, true, true, true);
        rigid_body.set_enabled_rotations(false, false, false, true);

        let interaction_groups = match movable.kind {
            EntityKind::SpacePlayer => { InteractionGroups { memberships: SPACE_PLAYER, filter: BOUNDARY | BALL | SPACE_WALL | SPACE_PLAYER | TIME_PLAYER } },
            EntityKind::TimePlayer => { InteractionGroups { memberships: TIME_PLAYER, filter: BOUNDARY | BALL | TIME_WALL | SPACE_PLAYER | TIME_PLAYER } },
            _ => panic!("Kind not allowed")
        };

        let collider = ColliderBuilder::cuboid(dimension.x / 2.0, dimension.y / 2.0, dimension.z / 2.0)
            .restitution(restitution)
            .collision_groups(interaction_groups)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .friction(2.0)
            .friction_combine_rule(CoefficientCombineRule::Max)
            .mass(30.0)
            .user_data(identifier)
            .build();
    
        let rigid_body_handle = self.rigid_body_set.insert(rigid_body);
        let collider_handle = self.collider_set.insert_with_parent(collider, rigid_body_handle, &mut self.rigid_body_set);
        self.references.insert(identifier, create_reference(rigid_body_handle, collider_handle, dimension.z / 2.0, movable));
    }

    pub fn get_player_ball_collision_events(&self) -> &Vec<PlayerBallCollision> {
        &self.player_ball_collisions
    }

    pub fn get_player_wall_collision_events(&self) -> &Vec<PlayerWallCollision> {
        &self.player_wall_collisions
    }

    pub fn get_ball_wall_collision_events(&self) -> &Vec<BallWallCollision> {
        &self.ball_wall_collisions
    }

    pub fn reset_ball_colliders(&mut self, ball_name: &str) -> Result<(), String> {
        self.update_ball_collision_group(ball_name, BallCollisionGroup::Full)
    }

    fn update_ball_collision_group(&mut self, ball_name: &str, group: BallCollisionGroup) -> Result<(), String> {
        let identifier = get_identifier(ball_name);
        if let Some(reference) = self.references.get_mut(&identifier) {
            reference.partner_name = None;
            if let Some(collider) = self.collider_set.get_mut(reference.collider_handle) {
                let filter: Group = match group {
                    BallCollisionGroup::Full => BOUNDARY | BALL | SPACE_PLAYER | TIME_PLAYER,
                    BallCollisionGroup::OnlySpace => BOUNDARY | BALL | SPACE_PLAYER,
                    BallCollisionGroup::OnlyTime => BOUNDARY | BALL | TIME_PLAYER,
                    BallCollisionGroup::Empty => BOUNDARY
                };

                collider.set_collision_groups(InteractionGroups { memberships: BALL, filter });
                Ok(())
            } else {
                Err(format!("Could not find collider {}", ball_name))
            }
        } else {
            Err(format!("Could not find reference {}", ball_name))
        }
    }

    pub fn update_movable(&mut self, movable: &mut Movable) -> Result<bool, String> {
        let identifier = get_identifier(&movable.name);
        if let Some(handle) = self.references.get(&identifier) {
            let rigid_body = &self.rigid_body_set[handle.rigid_body_handle];
            let position = rigid_body.translation();
            let velocity = rigid_body.linvel();

            let velocity = Velocity { x: Self::dead_zone(velocity.x), y: Self::dead_zone(velocity.y), z: Self::dead_zone(velocity.z) };
            let position = Position { x: position.x, y: position.y, z: Self::dead_zone(position.z - handle.half_z) };
            let mut updated = false;
            if movable.velocity.delta_norm(&velocity) > DEAD_ZONE {
                movable.velocity = velocity;
                updated = true;
            }

            if movable.position.delta_norm(&position) > DEAD_ZONE {
                movable.position = position;
                updated = true;
            }

            Ok(updated)

        } else {
            Err(format!("Reference with name {} was not found", movable.name))
        }
    }

    pub fn delete(&mut self, name: &str) -> Result<Option<String>, String> {
        let identifier = get_identifier(name);
        let reference = self.find_reference_by_name(name)?;
        let joint = reference.joint;
        let rigid_body_handle = reference.rigid_body_handle;

        let mut related_entity_name: Option<String> = None;

        if let Some(joint_handle) = joint {
            let (deleted_name1, deleted_name2) = self.delete_joint(joint_handle, name)?;
            related_entity_name = Some(if deleted_name1 == name { deleted_name2 } else { deleted_name1 });
        }

        self.rigid_body_set.remove(
            rigid_body_handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true);

        self.references.remove(&identifier);
        
        Ok(related_entity_name)
    }

    pub fn step(&mut self, dt: f32) {
        self.physics_pipeline.step(
            &vector![0.0, 0.0, -GRAVITY],
            &IntegrationParameters { dt: dt, ..self.integration_parameters },
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &self.physics_hooks,
            &self.event_handler
        );

        self.store_collision_events();
    }

    pub fn set_velocity(&mut self, name: &String, velocity: Velocity) -> Result<(), String> {
        let rigid_body = self.find_mut_rigid_body_by_name(name)?;
        rigid_body.set_linvel(vector![velocity.x, velocity.y, velocity.z], true);
        Ok(())
    }
    
    pub fn apply_impulse(&mut self, name: &str, velocity: Velocity) -> Result<(), String> {
        let rigid_body = self.find_mut_rigid_body_by_name(name)?;
        rigid_body.apply_impulse(vector![velocity.x, velocity.y, velocity.z], true);
        Ok(())
    }

    pub fn catch_ball(&mut self, player_name: &str, ball_name: &str) -> Result<(), String> {
        let ball_reference = self.find_reference_by_name(ball_name)?;
        let player_reference = self.find_reference_by_name(player_name)?;
        
        if ball_reference.joint.is_some() {
            Err(format!("Ball {} has already a joint when it collided with player {}", ball_name, player_name))
        } else if player_reference.joint.is_some() {
            Err(format!("Player {} has already a joint when it collided with ball {}", player_name, ball_name))
        } else {
            let offset_x = if player_reference.kind == EntityKind::SpacePlayer { 0.2 } else { -0.2 };
            self.update_ball_collision_group(ball_name, BallCollisionGroup::Empty)?;
            let offset = Position { x: offset_x, y: PLAYER_DIMENSION.y * -0.2, z: PLAYER_DIMENSION.z * -0.2 };
            self.sync_positions(player_name, ball_name, &offset)?;
            
            let joint = FixedJointBuilder::new()
                .local_anchor1(point![offset.x, offset.y, offset.z])
                .local_anchor2(point![0.0, 0.0, 0.0]);
        
            let ball_ref = self.find_reference_by_name(ball_name)?;
            let player_ref = self.find_reference_by_name(player_name)?;
            let joint_handle = self.impulse_joint_set.insert(player_ref.rigid_body_handle, ball_ref.rigid_body_handle, joint, true);
        
            let ball_mutable_reference = self.find_mut_reference_by_name(ball_name)?;
            ball_mutable_reference.joint = Some(joint_handle);
        
            let player_mutable_reference = self.find_mut_reference_by_name(player_name)?;
            player_mutable_reference.joint = Some(joint_handle);
        
            Ok(())
        }
    }

    pub fn throw_ball(&mut self, player_name: &str, kind: EntityKind, destination: Position) -> Result<(), String> {
        let ball_option = self.get_joint_partner(player_name)?;
        if let Some((ball_name, joint_handle)) = ball_option {
            self.delete_joint(joint_handle, player_name)?;
            let collision_group = if kind == EntityKind::SpacePlayer { BallCollisionGroup::OnlyTime } else { BallCollisionGroup::OnlySpace };
            
            self.update_ball_collision_group(&ball_name, collision_group)?;
            let ball_rigid_body = self.find_rigid_body_by_name(&ball_name)?;
            let ball_position = ball_rigid_body.translation();
            if let Some(impulse) = compute_impulse_to_target(Position { x: ball_position[0], y: ball_position[1], z: ball_position[2] }, destination, ball_rigid_body.mass()) {
                match self.find_mut_reference(ball_rigid_body.user_data, &ball_name) {
                    Ok(ball_reference) => {
                        ball_reference.partner_name = Some(String::from(player_name));
                        self.apply_impulse(&ball_name, impulse)?;
                        Ok(())
                    },
                    Err(message) => { Err(message) }
                }
            } else {
                Err(format!("Could not calculate throw trajectory for player {}", player_name))
            }
        } else {
            Err(format!("Player {} was not holding a ball", player_name))
        }
    }

    fn get_joint_partner(&self, name: &str) -> Result<Option<(String, ImpulseJointHandle)>, String> {
        let reference = self.find_reference_by_name(name)?;
        if let Some(joint_handle) = reference.joint {
            let joint = self.find_joint(joint_handle, name)?;
            let partner_rigid_body_handle = if reference.rigid_body_handle == joint.body1 { joint.body2 } else { joint.body1 };
            let partner_rigid_body = self.find_rigid_body(partner_rigid_body_handle, name)?;
            let partner_reference = self.find_reference(partner_rigid_body.user_data, name)?;
            Ok(Some((partner_reference.name.clone(), joint_handle)))
        } else {
            Ok(None)
        }
    }

    fn detect_collision_type(&self, reference_1: ReferenceResult, reference_2: ReferenceResult) -> Result<CollisionType, String> {
        match (reference_1, reference_2) {
            (ReferenceResult::PlayerReference(ref1), ReferenceResult::Wall) |
            (ReferenceResult::Wall, ReferenceResult::PlayerReference(ref1)) => {
                Ok(CollisionType::PlayerWall(PlayerWallCollision { player_name: ref1.name.clone() }))
            },
            (ReferenceResult::PlayerReference(ref1), ReferenceResult::BallReference(ref2)) |
            (ReferenceResult::BallReference(ref2), ReferenceResult::PlayerReference(ref1)) => {
                if let Some(ball_body) = self.rigid_body_set.get(ref2.rigid_body_handle) {
                    let velocity = ball_body.linvel();
                    if let Some(ball_collider) = self.collider_set.get(ref2.collider_handle) {
                        match self.find_reference(ball_collider.user_data, &ref2.name) {
                            Ok(ball_reference) => {
                                Ok(CollisionType::PlayerBall(PlayerBallCollision {
                                    player_name: ref1.name.clone(),
                                    ball_name: ref2.name.clone(),
                                    velocity: Velocity { x: velocity[0], y: velocity[1], z: velocity[2] },
                                    ball_has_rebounded: ball_is_in_collision_group(ball_collider, BallCollisionGroup::Full),
                                    tossed_by_name: ball_reference.partner_name.clone()
                                }))
                            },
                            Err(message) => { Err(message) }
                        }
                    } else {
                        Err(format!("Could not resolve collider of ball {} that collided with player {}", ref2.name, ref1.name))
                    }
                } else {
                    // TODO: the ball must be removed?
                    Err(format!("Could not resolve rigid body of ball {} that collided with player {}", ref2.name, ref1.name))
                }
            },
            (ReferenceResult::Wall, ReferenceResult::BallReference(ball)) |
            (ReferenceResult::Floor, ReferenceResult::BallReference(ball)) |
            (ReferenceResult::BallReference(ball), ReferenceResult::Floor) |
            (ReferenceResult::BallReference(ball), ReferenceResult::Wall) => {
                Ok(CollisionType::BallWall(BallWallCollision { ball_name: ball.name.clone() }))
            },
            _ => { Ok(CollisionType::Ignore) }
        }
    }

    fn get_reference_result(&self, handle_collider: ColliderHandle) -> ReferenceResult {
        if let Some(collider) = self.collider_set.get(handle_collider) {
            if collider.translation().z < 0.0 {
                return ReferenceResult::Floor;
            }

            if collider.user_data != 0 {
                if let Some(reference) = self.references.get(&collider.user_data) {
                    if reference.kind == EntityKind::Ball {
                        return ReferenceResult::BallReference(reference);
                    } else {
                        return ReferenceResult::PlayerReference(reference);
                    }
                } else {
                    return ReferenceResult::Wall;
                }
            }
        }

        ReferenceResult::Wall
    }

    fn store_collision_events(self: &mut Self) {
        self.player_ball_collisions.clear();
        self.player_wall_collisions.clear();
        self.ball_wall_collisions.clear();

        while let Ok(event) = self.collision_receiver.try_recv() {
            match event {
                CollisionEvent::Started(collider_handle_1, collider_handle_2, _flags) => {
                    let reference_1 = self.get_reference_result(collider_handle_1);
                    let reference_2 = self.get_reference_result(collider_handle_2);

                    match self.detect_collision_type(reference_1, reference_2) {
                        Ok(CollisionType::PlayerBall(player_ball_collision)) => {
                            self.player_ball_collisions.push(player_ball_collision);
                        },
                        Ok(CollisionType::PlayerWall(player_wall_collision)) => {
                            self.player_wall_collisions.push(player_wall_collision);
                        },
                        Ok(CollisionType::BallWall(ball_wall_collision)) => {
                            self.ball_wall_collisions.push(ball_wall_collision);
                        },
                        Err(message) => {
                            log::error!("Could not detect collision type: {}", message)
                        },
                        _ => { }
                    }
                },
                CollisionEvent::Stopped(_, _, _) => {}
            }
        }
    }

    fn dead_zone(value: f32) -> f32 {
        if value.abs() < DEAD_ZONE {
            0.0
        } else {
            value
        }
    }

    fn add_boundaries(&mut self, dimension: Dimension) {
        let wall_thickness: f32 = 10.0;
        let position: Position = Position::ZERO;

        // floor
        self.add_boundary(
            Position { z: -wall_thickness, ..position },
            Dimension { x: dimension.x + wall_thickness * 2.0, y: dimension.y + wall_thickness * 2.0, z: wall_thickness },
            (BOUNDARY, Group::all()));

        // ceiling
        self.add_boundary(
            Position { z: dimension.z, ..position },Dimension { x: dimension.x + wall_thickness * 2.0, y: dimension.y + wall_thickness * 2.0, z: wall_thickness },
            (BOUNDARY, Group::all()));

        // left wall
        self.add_boundary(
            Position { x: -(dimension.x + wall_thickness) / 2.0, ..position },
            Dimension { x: wall_thickness, y: dimension.y + wall_thickness * 2.0, ..dimension },
            (BOUNDARY, Group::all()));

        // right wall
        self.add_boundary(
            Position { x: (dimension.x + wall_thickness) / 2.0, ..position },
            Dimension { x: wall_thickness, y: dimension.y + wall_thickness * 2.0, ..dimension },
            (BOUNDARY, Group::all()));

        // front wall
        self.add_boundary(
            Position { y: -(dimension.y + wall_thickness) / 2.0, ..position },
            Dimension { x: dimension.x, y: wall_thickness, ..dimension },
            (BOUNDARY, Group::all()));

        // back wall
        self.add_boundary(
            Position { y: (dimension.y + wall_thickness) / 2.0, ..position },
            Dimension { x: dimension.x, y: wall_thickness, ..dimension },
            (BOUNDARY, Group::all()));

        // Space wall
        self.add_boundary(
            Position { x: wall_thickness / 2.0 + 0.3, ..position },
            Dimension { x: wall_thickness, ..dimension },
            (SPACE_WALL, SPACE_PLAYER));

        // Time wall
        self.add_boundary(
            Position { x: -(wall_thickness / 2.0 + 0.3), ..position },
            Dimension { x: wall_thickness, y: dimension.y / 2.0, ..dimension },
            (TIME_WALL, TIME_PLAYER));
    }

    fn add_boundary(&mut self, position: Position, dimension: Dimension, groups: (Group, Group)) {
        let boundary_position = vector![position.x, position.y, position.z + dimension.z / 2.0];
    
        let mut collider_builder = 
            ColliderBuilder::cuboid(dimension.x / 2.0, dimension.y / 2.0, dimension.z / 2.0)
            .friction(2.0)
            .translation(boundary_position);

        let (memberships, filter) = groups;
        collider_builder = collider_builder.collision_groups(InteractionGroups { memberships, filter });
        
        let collider = collider_builder.build();
        self.collider_set.insert(collider);
    }

    fn find_reference_by_name(&self, name: &str) -> Result<&Reference, String> {
        let identifier = get_identifier(name);
        self.find_reference(identifier, name)
    }

    fn find_reference(&self, identifier: u128, name: &str) -> Result<&Reference, String> {
        if let Some(reference) = self.references.get(&identifier) {
            Ok(reference)
        } else {
            Err(format!("Could not find reference for {}", name))
        }
    }

    fn find_joint(&self, joint_handle: ImpulseJointHandle, name: &str) -> Result<&ImpulseJoint, String> {
        if let Some(joint) = self.impulse_joint_set.get(joint_handle) {
            Ok(joint)
        } else {
            Err(format!("Could not find joint for {}", name))
        }
    }

    fn find_mut_reference_by_name(&mut self, name: &str) -> Result<&mut Reference, String> {
        let identifier = get_identifier(name);
        self.find_mut_reference(identifier, name)
    }

    fn find_mut_reference(&mut self, identifier: u128, name: &str) -> Result<&mut Reference, String> {
        if let Some(reference) = self.references.get_mut(&identifier) {
            Ok(reference)
        } else {
            Err(format!("Could not find reference for {}", name))
        }
    }

    fn find_rigid_body_by_name(&self, name: &str) -> Result<&RigidBody, String> {
        let reference = self.find_reference_by_name(name)?;
        self.find_rigid_body(reference.rigid_body_handle, name)
    }

    fn find_rigid_body(&self, rigid_body_handle: RigidBodyHandle, name: &str) -> Result<&RigidBody, String> {
        if let Some(rigid_body) = self.rigid_body_set.get(rigid_body_handle) {
            Ok(rigid_body)
        } else {
            Err(format!("Could not find rigid body for {}", name))
        }
    }

    fn find_mut_rigid_body_by_name(&mut self, name: &str) -> Result<&mut RigidBody, String> {
        let reference = self.find_reference_by_name(name)?;
        self.find_mut_rigid_body(reference.rigid_body_handle, name)
    }

    fn find_mut_rigid_body(&mut self, rigid_body_handle: RigidBodyHandle, name: &str) -> Result<&mut RigidBody, String> {
        if let Some(rigid_body) = self.rigid_body_set.get_mut(rigid_body_handle) {
            Ok(rigid_body)
        } else {
            Err(format!("Could not find rigid body for {}", name))
        }
    }
    
    fn delete_joint(&mut self, impulse_joint: ImpulseJointHandle, name: &str) -> Result<(String, String), String> {
        if let Some(joint) = self.impulse_joint_set.remove(impulse_joint, true) {
            let reference_name1 = self.release_joint_from_rigid_body(joint.body1, name)?;
            let reference_name2 = self.release_joint_from_rigid_body(joint.body2, name)?;
            Ok((reference_name1, reference_name2))
        } else {
            Err(String::from("Could not find joint on deletion"))
        }
    }

    fn release_joint_from_rigid_body(&mut self, rigid_body_handle: RigidBodyHandle, name: &str) -> Result<String, String> {
        let rigid_body = self.find_mut_rigid_body(rigid_body_handle, name)?;
        let identifier = rigid_body.user_data;
        let reference = self.find_mut_reference(identifier, name)?;
        reference.joint = None;

        let reference_name = reference.name.clone();
        match reference.kind {
            EntityKind::Ball => {
                match self.update_ball_collision_group(&reference_name, BallCollisionGroup::Full) {
                    _ => { }
                }
            },
            _ => {}
        };

        Ok(reference_name)
    }
    
    fn sync_positions(&mut self, parent_name: &str, child_name: &str, offset: &Position) -> Result<(), String> {
        let parent_rigid_body = self.find_rigid_body_by_name(parent_name)?;
        let parent_translation = parent_rigid_body.translation().clone();

        let mutable_child_rigid_body = self.find_mut_rigid_body_by_name(child_name)?;
        mutable_child_rigid_body.set_translation(vector![parent_translation[0] + offset.x, parent_translation[1] + offset.y, parent_translation[2] + offset.z], true);
        mutable_child_rigid_body.set_linvel(vector![0.0, 0.0, 0.0], true);

        Ok(())
    }
}

fn compute_impulse_to_target(start: Position, target: Position, mass: f32) -> Option<Velocity> {
    let dx = target.x - start.x;
    let dy = target.y - start.y;
    let dz = target.z - start.z;
    const MIN_NORM: f32 = 1e-5;
    
    let diagonal: f32 = (FIELD_DIMENSION.x * FIELD_DIMENSION.x + FIELD_DIMENSION.y * FIELD_DIMENSION.y).sqrt();
    let distance = start.delta_norm(&target);
    let angle_deg = (45.0 * (distance / diagonal)).min(45.0).max(0.0);
    let velocity_correction = 1.0 + ((1.0 - (distance / diagonal)) * 0.5);
    
    // Horizontal distance in XY plane
    let horizontal = Vector3D { x: dx, y: dy, z: 0.0 };
    let dx_y = horizontal.norm();
    if dx_y < MIN_NORM {
        return None; // Can't solve without horizontal distance
    }

    let angle_rad = angle_deg.to_radians();
    let tan_theta = angle_rad.tan();
    let cos_theta = angle_rad.cos();

    // Solve: dz = dx_y * tan(θ) - (g * dx_y²) / (2 * v² * cos²(θ))
    let denom = 2.0 * (dx_y * tan_theta - dz) * cos_theta.powi(2);
    if denom <= 0.0 {
        return None; // No valid solution
    }

    let v = ((GRAVITY * dx_y * dx_y) / denom).sqrt();
    let horizontal_dir = horizontal.try_normalize(MIN_NORM)?;
    let sin_theta = angle_rad.sin();

    let launch_dir = Vector3D {
        x: horizontal_dir.x * cos_theta,
        y: horizontal_dir.y * cos_theta,
        z: sin_theta,
    }
    .try_normalize(MIN_NORM)?;

    // Velocity and impulse
    let velocity = Vector3D {
        x: launch_dir.x * v,
        y: launch_dir.y * v,
        z: launch_dir.z * v,
    };

    let impulse = Vector3D {
        x: velocity.x * mass * velocity_correction,
        y: velocity.y * mass * velocity_correction,
        z: velocity.z * mass * velocity_correction,
    };

    Some(impulse)
}

fn ball_is_in_collision_group(ball_collider: &Collider, group: BallCollisionGroup) -> bool {
    let filter = ball_collider.collision_groups().filter;
    match group {
        BallCollisionGroup::Full => filter == BOUNDARY | BALL | SPACE_PLAYER | TIME_PLAYER,
        BallCollisionGroup::OnlySpace => filter == BOUNDARY | BALL | SPACE_PLAYER,
        BallCollisionGroup::OnlyTime => filter == BOUNDARY | BALL | TIME_PLAYER,
        BallCollisionGroup::Empty => filter == BOUNDARY
    }
}

fn create_reference(rigid_body_handle: RigidBodyHandle, collider_handle: ColliderHandle, half_z: f32, movable: &Movable) -> Reference {
    Reference {
        rigid_body_handle,
        collider_handle,
        half_z,
        name: movable.name.clone(),
        kind: movable.kind.clone(),
        joint: None,
        partner_name: None
    }
}

fn get_identifier(name: &str) -> u128 {
    let bytes = name.as_bytes();
    if bytes.len() > 16 {
        0
    } else {
        let mut result = 0u128;
        for &b in bytes {
            result = (result << 8) | b as u128;
        }

        result
    }
}

#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn process_should_track_player() {
        let mut system = PhysicsEngine::new(Dimension { x: 10.0, y: 10.0, z: 10.0 });
        let mut player = Movable {
            name: String::from("Xyz"),
            kind: EntityKind::SpacePlayer,
            position: Position::ZERO,
            velocity: Velocity::ZERO,
            state: EntityState::ReadyToMove
        };

        let dimension = Dimension { x: 1.0, y: 1.0, z: 1.0 };
        let restitution: f32 = 0.8;

        system.upsert_player(&player, dimension, restitution);
        system.step(0.05);
        if let Err(_) = system.update_movable(&mut player) {
            assert!(false);
        }

        assert!(true);
    }

    #[test]
    fn process_should_track_ball() {
        let mut system = PhysicsEngine::new(Dimension { x: 10.0, y: 10.0, z: 10.0 });
        let mut player = Movable {
            name: String::from("Ball1"),
            kind: EntityKind::Ball,
            position: Position { z: 0.8, ..Position::ZERO },
            velocity: Velocity::ZERO,
            state: EntityState::ReadyToMove
        };

        let dimension = Dimension { x: 0.2, y: 0.2, z: 0.2 };
        let restitution: f32 = 0.8;

        system.upsert_player(&player, dimension, restitution);
        system.step(0.05);
        if let Err(_) = system.update_movable(&mut player) {
            assert!(false);
        }

        assert!(true);
    }
}
