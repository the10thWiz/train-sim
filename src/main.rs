mod terrain;
pub mod util;

use std::{f32::consts::PI, fs::File};

use bevy::{app::AppExit, prelude::*, tasks::AsyncComputeTaskPool};
use smooth_bevy_cameras::{controllers::fps::*, LookTransformPlugin};
use terrain::{TerrainChunk, TerrainLoader};

fn main() {
    // simple_log::quick!();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(terrain::TerrainPlugin)
        .add_plugin(LookTransformPlugin)
        .add_plugin(FpsCameraPlugin::default())
        .add_startup_system(setup)
        .add_system(quit)
        .run();
}

#[allow(unused)]
fn generate_images() {
    AsyncComputeTaskPool::get()
        .spawn(async move {
            let mut n = 0;
            for mut terrain in TerrainChunk::from_images(
                "mountain_range_height_map.png",
                "mountain_range_color_map.png",
                64,
                1.,
                0.2,
            )
            .unwrap()
            {
                // Position ID
                terrain.move_mesh(Vec3::new(0., -16_000. * 0.2, 0.));
                let pos = (terrain.pos() / 64.).floor().as_ivec3();
                TerrainLoader {}
                    .write_chunk(
                        &mut File::create(format!("assets/chunks/{}_{}.chunk", pos.x, pos.z))
                            .unwrap(),
                        &terrain,
                    )
                    .unwrap();
                n += 1;
                if n % 32 == 0 {
                    log::info!("Written {n} chunks");
                }
            }
            log::info!("Chunk generation completed");
        })
        .detach();
}

fn setup(mut commands: Commands) {
    // generate_images();

    // light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 2., 0., 0.)),
        directional_light: DirectionalLight {
            shadows_enabled: false,
            ..Default::default()
        },
        ..Default::default()
    });

    commands
        .spawn(Camera3dBundle::default())
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                translate_sensitivity: 50.,
                ..Default::default()
            },
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(32., 0., 32.),
            Vec3::Y,
        ));
}

fn quit(input: Res<Input<KeyCode>>, mut quit: EventWriter<AppExit>) {
    if (input.pressed(KeyCode::RControl) || input.pressed(KeyCode::LControl))
        && input.pressed(KeyCode::C)
    {
        quit.send_default();
    }
}
