mod camera;
mod ui;

use std::f32::consts::PI;
use std::ops::Range;
use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
    sprite::Mesh2dHandle,
    time::FixedTimestep,
};
use bevy::utils::HashMap;
use bevy_spatial::{KDTreeAccess2D, KDTreePlugin2D, SpatialAccess};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use leafwing_input_manager::prelude::*;
use lerp;
use lerp::Lerp;
use rand::prelude::ThreadRng;
use rand::{random, Rng};
use crate::camera::ControlledCamera;
use crate::ui::UI;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;
const PARTICLE_SCALE: Vec3 = Vec2::splat(20.).extend(0.);
const PARTICLE_COUNT: usize = 256;
const FAMILY_COUNT: usize = 16;
const BATCH_SIZE: usize = 128;
const POS_GEN_RANGE: Range<f32> = -1000.0..1000.0;
const TWO_PI: f32 = 2. * PI;
const MAX_INTERACTION_DIST: f32 = 100.;
const FAST_MODE: bool = false;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ControlledCamera)
        .add_plugin(KDTreePlugin2D::<Particle> { ..default() })
        .register_type::<ParticleMeta>()
        // .add_plugin(ResourceInspectorPlugin::<ParticleMeta>::default())
        .insert_resource(ParticleMeta::random(FAMILY_COUNT))
        // .add_plugin(UI)
        .add_startup_system(setup_graphics)
        .add_startup_system(spawn_particles)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(update_velocity)
                .with_system(apply_velocity.before(update_velocity))
        )
        .run();
}

fn setup_graphics(mut commands: Commands) {
    //background
    commands.insert_resource(ClearColor(Color::rgb(0.9, 0.9, 0.9)))
}

fn spawn_particles(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>,
                   mut materials: ResMut<Assets<ColorMaterial>>, meta: Res<ParticleMeta>)
{
    let circle = meshes.add((shape::Circle{radius: 0.5, vertices: 16}).into());
    let mut rng = rand::thread_rng();
    for i in 0..meta.families.len(){
        let mat = materials.add(ColorMaterial::from(meta.colors[i]));
        for _ in 0..PARTICLE_COUNT {
            spawn_particle(
                &mut commands,
                i,
                circle.clone(),
                mat.clone(),
                Vec3::new(
                    rng.gen_range(POS_GEN_RANGE),
                    rng.gen_range(POS_GEN_RANGE),
                    rng.gen_range(-0.01..0.01))
            )
        }
    }
}

fn spawn_particle(commands: &mut Commands,
                  family: ParticleFamilyIdx,
                  mesh_handle: Handle<Mesh>,
                  material: Handle<ColorMaterial>,
                  pos: Vec3)
{
    commands.spawn(Particle(family))
        .insert(MaterialMesh2dBundle{
            material,
            mesh: Mesh2dHandle(mesh_handle),
            transform: Transform::from_translation(pos).with_scale(PARTICLE_SCALE),
            ..default()
        })
        .insert(Velocity(default()))
    ;
}

fn apply_velocity(mut query: Query<(&mut Transform, &mut Velocity)>) {
    query.par_for_each_mut(BATCH_SIZE, |(mut transform, mut velocity)| {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
        // velocity.0 = default()
    })
}

type UpdateQuery<'a> =(Entity, &'a Transform, &'a mut Velocity, &'a Particle);

fn update_velocity(mut update_query: Query<UpdateQuery>,
                   particle_query: Query<&Particle>,
                   tree: Res<KDTreeAccess2D<Particle>>,
                   meta: Res<ParticleMeta>)
{
    update_query.par_for_each_mut(BATCH_SIZE, |(target_ent, target_tx, mut target_vel, target)| {
        for (neighbor_pos, neighbor_ent) in tree.within_distance(target_tx.translation, MAX_INTERACTION_DIST) {
            if target_ent == neighbor_ent {
                continue
            }
            if let Ok(neighbor_particle) = particle_query.get(neighbor_ent) {
                let mut delta = neighbor_pos - target_tx.translation;
                let len = delta.length();
                if len <= f32::EPSILON { //todo collision event?
                    continue;
                }
                let force = meta.families[target.0].attraction(&meta.families[neighbor_particle.0], len / MAX_INTERACTION_DIST);
                let delta_force = delta.normalize() * force;
                target_vel.0 += delta_force.truncate();
            }
        }
    })
}

type ParticleFamilyIdx = usize;

#[derive(Component)]
struct Particle(ParticleFamilyIdx);

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Reflect, Resource, Default)]
#[reflect(Resource)]
struct ParticleMeta {
    colors: Vec<Color>,
    families: Vec<FamilyProperties>,
}

impl ParticleMeta {
    fn random(count: usize) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            colors: (0..count).map(|_|Color::rgb(
                rng.gen_range(0.0..1.),
                rng.gen_range(0.0..1.),
                rng.gen_range(0.0..1.)))
                .collect(),
            families: (0..count).map(|idx| FamilyProperties::random(idx, &mut rng)).collect()
        }
    }
}

#[derive(Reflect, FromReflect)]
struct ActivationProps {
    start: f32,
    slope: f32,
    end: f32,
}

impl ActivationProps {
    fn scale(&self, mul: f32) -> Self {
        Self {
            start: self.start * mul,
            slope: self.slope * mul,
            end: self.end * mul,
        }
    }

    fn random(rng: &mut ThreadRng) -> Self {
        Self {
            start: rng.gen_range(0.0..2.0),
            slope: rng.gen_range(-10.0 .. 10.0),
            end: rng.gen_range(MAX_INTERACTION_DIST/2.0 .. MAX_INTERACTION_DIST),
        }
    }
}

#[derive(Reflect, FromReflect)]
struct CurveProps {
    x_mul: f32,
    y_mul: f32,
    x_offset: f32,
}

impl CurveProps {
    fn scale(&self, mul: f32) -> Self {
        Self {
            x_mul: self.x_mul * mul,
            y_mul: self.y_mul * mul,
            x_offset: self.x_offset * mul,
        }
    }

    fn random(rng: &mut ThreadRng) -> Self {
        Self {
            x_mul: rng.gen_range(-2.0..2.0),
            y_mul: rng.gen_range(-10.0..10.0),
            x_offset: rng.gen_range(0.0..TWO_PI),
        }
    }
}

#[derive(Reflect, FromReflect)]
enum Curve {
    Sin(CurveProps),
    Cos(CurveProps),
    Activation(ActivationProps)
}

impl Curve {
    fn scale(&self, mul: f32) -> Self {
        match self {
            Curve::Sin(props) => Curve::Sin(props.scale(mul)),
            Curve::Cos(props) => Curve::Cos(props.scale(mul)),
            Curve::Activation(props) => Curve::Activation(props.scale(mul))
        }
    }

    fn random(rng: &mut ThreadRng) -> Curve {
        if FAST_MODE {
            Curve::Activation(ActivationProps::random(rng))
        } else {
            match rng.gen_bool(0.5) {
                true => Curve::Cos(CurveProps::random(rng)),
                false => Curve::Sin(CurveProps::random(rng)),
            }
        }
    }

    fn sample(&self, factor: f32) -> f32 {
        match self {
            Curve::Sin(props) => ((0.0.lerp_bounded(TWO_PI, factor) + props.x_offset) * props.x_mul).sin() * props.y_mul,
            Curve::Cos(props) => ((0.0.lerp_bounded(TWO_PI, factor) + props.x_offset) * props.x_mul).cos() * props.y_mul,
            Curve::Activation(props) => match factor > props.start {
                true => (factor - props.start) * props.slope,
                false => 0.
            }
        }
    }
}

#[derive(Reflect, FromReflect)]
struct FamilyProperties {
    ancestry: Vec<f32>,
    curves: Vec<Vec<Curve>>,
}

impl FamilyProperties {
    fn random(family_idx: ParticleFamilyIdx, rng: &mut ThreadRng) -> Self {
        Self {
            ancestry: (0..FAMILY_COUNT).map(|idx| if family_idx == idx { 1. } else { 0. }).collect(),
            curves: (0..FAMILY_COUNT).map(|_|vec![Curve::random(rng)]).collect()
        }
    }

    fn combine(&self, other: &Self, weights: (f32, f32)) -> Self {
        Self {
            ancestry: std::iter::zip(self.ancestry.iter(), other.ancestry.iter())
                .map(|(a, b)| (a * weights.0) + (b * weights.1))
                .collect(),
            curves: self.curves.iter().enumerate().map(|(idx, curves)|
                curves.iter().map(|sub_curve| sub_curve.scale(weights.0))
                    .chain(other.curves[idx].iter().map(|sub_curve| sub_curve.scale(weights.1)))
                    .collect()
            ).collect()
        }
    }

    fn attraction(&self, other: &Self, distance: f32) -> f32 {
        std::iter::zip(self.curves.iter(), other.ancestry.iter()).map(|(curves, weight)| -> f32 {
            return if *weight == 0. {
                0.
            } else {
                weight * curves.iter().map(|curve| curve.sample(distance)).sum::<f32>()
            }
        }).sum()
    }
}