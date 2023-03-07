use std::time::{Duration, Instant};

use bevy::{asset::LoadState, prelude::*};
use smooth_bevy_cameras::LookTransform;

use super::TerrainChunk;

#[derive(Debug, Resource)]
pub struct ChunkedLoader {
    chunk_size: f32,
    render_dist_sq: usize,
    loading: Vec<(IVec2, Handle<TerrainChunk>)>,
    failed: Vec<IVec2>,
    material: Handle<StandardMaterial>,
    last_update: Instant,
    update_tick: Duration,
}

#[derive(Debug, Component)]
pub struct Debug;

#[derive(Debug, Component)]
pub struct TerrainComponent {
    pos: IVec2,
}

impl ChunkedLoader {
    pub fn new(app: &mut App) -> Self {
        let material = app
            .world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(Color::rgba(1., 1., 1., 1.).into());
        let update_tick = Duration::from_millis(200);
        Self {
            chunk_size: 64.,
            render_dist_sq: 10 * 10,
            loading: vec![],
            failed: vec![],
            material,
            last_update: Instant::now() - update_tick,
            update_tick,
        }
    }

    #[allow(unused)]
    pub fn set_render_dist(&mut self, dist: usize) {
        self.render_dist_sq = dist * dist;
    }

    #[allow(unused)]
    pub fn time_since_last_update(&self) -> Duration {
        self.last_update.elapsed()
    }

    #[allow(unused)]
    pub fn set_update_tick(&mut self, time: Duration) {
        self.update_tick = time;
    }

    pub fn update(
        mut this: ResMut<Self>,
        chunks: Res<Assets<TerrainChunk>>,
        server: Res<AssetServer>,
        camera: Query<&LookTransform>,
        mut commands: Commands,
        loaded: Query<(&TerrainComponent, Entity)>,
    ) {
        // Avoid mutable deref if it hasn't been enough time
        if this.last_update.elapsed() < this.update_tick {
            return;
        }
        let this: &mut Self = this.as_mut();
        this.last_update = Instant::now();
        this.loading
            .retain(|(id, handle)| match server.get_load_state(handle) {
                LoadState::Loaded => {
                    let chunk = chunks.get(handle).unwrap();
                    log::info!("Loaded {}, from {} to {}", id, chunk.min, chunk.max);
                    commands
                        .spawn(MaterialMeshBundle {
                            mesh: chunk.mesh(),
                            transform: Transform::from_translation(chunk.pos()),
                            // transform: Transform::from_translation(
                            //     chunk.pos() - Vec3::new(0., 70., 0.),
                            // ) * Transform::from_scale(Vec3::new(1., 0., 1.))
                            //     * Transform::from_rotation(Quat::from_euler(
                            //         EulerRot::XYZ,
                            //         0.,
                            //         PI / 2.,
                            //         0.,
                            //     )),
                            material: this.material.clone(),
                            ..Default::default()
                        })
                        .insert(Debug)
                        .insert(TerrainComponent { pos: *id });
                    false
                }
                LoadState::NotLoaded => true,
                LoadState::Loading => true,
                LoadState::Failed => {
                    log::info!("Load Failed {}", id);
                    this.failed.push(*id);
                    false
                }
                LoadState::Unloaded => false,
            });
        let pos = camera.single().eye;
        let pos = (pos / this.chunk_size).floor().as_ivec3();
        let pos = IVec2::new(pos.x, pos.z);
        for pos in this.pos_iter(pos) {
            if loaded.iter().find(|(id, _)| id.pos == pos).is_none()
                && this.loading.iter().find(|(id, _)| *id == pos).is_none()
                && !this.failed.contains(&pos)
            {
                this.loading.push((
                    pos,
                    server.load(format!("chunks/{}_{}.chunk", pos.x, pos.y)),
                ));
            }
        }

        for (chunk, id) in loaded.iter() {
            let dir = chunk.pos - pos;
            if dir.dot(dir) > this.render_dist_sq as i32 + 3 {
                log::info!("Unloading {}", chunk.pos);
                commands.entity(id).despawn_recursive();
            }
        }
    }

    fn pos_iter(&self, pos: IVec2) -> impl Iterator<Item = IVec2> {
        let dist = self.render_dist_sq as i32;
        (-dist..dist)
            .flat_map(move |x| (-dist..dist).map(move |z| IVec2::new(x, z)))
            .filter(move |p| p.dot(*p) < dist)
            .map(move |p| p + pos)
    }
}
