use std::f32::consts::PI;

use bevy::{
    core_pipeline::{
        bloom::{BloomCompositeMode, BloomPrefilterSettings, BloomSettings},
        tonemapping::Tonemapping,
    },
    gltf::Gltf,
    pbr::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::view::ColorGrading,
};

#[derive(Resource, Default)]
struct GltfState {
    is_loaded: bool,
    handle: Handle<Gltf>,
}

#[derive(Component)]
struct Patched;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
        })
        .insert_resource(ClearColor(Color::GRAY))
        .insert_resource(GltfState::default())
        .add_startup_system(load_scene)
        .add_startup_system(hide_cursor)
        .add_startup_system(setup_camera_lights)
        .add_startup_system(setup_text)
        .add_system(patch_loaded_scene)
        .add_system(update_camera)
        .run();
}

fn load_scene(asset_server: Res<AssetServer>, mut gltf_state: ResMut<GltfState>) {
    gltf_state.handle = asset_server.load(String::from("ruins/scene.gltf"));
}

fn hide_cursor(mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    window.cursor.visible = false;
}

fn setup_camera_lights(mut commands: Commands) {
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 2.0,
        maximum_distance: 30.0,
        ..default()
    }
    .build();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: PerspectiveProjection {
                fov: PI / 8.0,
                ..default()
            }
            .into(),
            transform: Transform::from_xyz(0.0, 1.0, 1.0),
            tonemapping: Tonemapping::SomewhatBoringDisplayTransform,
            color_grading: ColorGrading {
                exposure: -0.5,
                post_saturation: 1.2,
                ..default()
            },
            ..default()
        },
        FogSettings {
            falloff: FogFalloff::Linear {
                start: 14.0,
                end: 35.0,
            },
            color: Color::DARK_GRAY * 0.85,
            ..default()
        },
        BloomSettings {
            intensity: 0.35,
            high_pass_frequency: 0.75,
            ..default()
        },
    ));

    // Campfire light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        point_light: PointLight {
            color: Color::rgb(1.0, 0.8, 0.2),
            intensity: 20.,
            range: 15.0,
            shadows_enabled: true,
            radius: 0.01,
            ..default()
        },
        ..default()
    });

    // Cyan lights for glass reflection
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(3.5, 1.0, 1.0),
        point_light: PointLight {
            color: Color::rgb(0.2, 0.8, 1.0),
            intensity: 5.,
            range: 10.0,
            radius: 0.01,
            ..default()
        },
        ..default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(1.0, 1.0, -3.5),
        point_light: PointLight {
            color: Color::rgb(0.2, 0.8, 1.0),
            intensity: 5.,
            range: 10.0,
            radius: 0.01,
            ..default()
        },
        ..default()
    });

    // Moonlight
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(0.98, 0.95, 0.82),
            illuminance: 4000.0,
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config,
        transform: Transform::from_xyz(0.25, 1.0, 1.5)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PlayfairDisplay-Regular.ttf");
    commands.spawn((TextBundle::from_section(
        "Original Scene: “Forest Ruins” by Casey Hardy (CC Attribution)",
        TextStyle {
            font,
            font_size: 16.0,
            color: Color::ANTIQUE_WHITE,
        },
    )
    .with_style(Style {
        position_type: PositionType::Absolute,
        position: UiRect {
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        ..default()
    }),));
}

fn patch_loaded_scene(
    mut commands: Commands,
    gltf_assets: Res<Assets<Gltf>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gltf_state: ResMut<GltfState>,
    named_entities: Query<(&Name, Entity), Without<Patched>>,
) {
    if !gltf_state.is_loaded {
        if let Some(gltf) = gltf_assets.get(&gltf_state.handle) {
            if let Some(stained_material_handle) = gltf.named_materials.get("stained".into()) {
                if let Some(mut stained_material) = materials.get_mut(stained_material_handle) {
                    stained_material.alpha_mode = AlphaMode::Multiply;
                    stained_material.fog_enabled = false;
                    stained_material.unlit = true;
                }
            }

            if let Some(stained_material_handle) =
                gltf.named_materials.get("stained-clearcoat".into())
            {
                if let Some(mut stained_material) = materials.get_mut(stained_material_handle) {
                    stained_material.alpha_mode = AlphaMode::Add;
                    stained_material.depth_bias = 0.3;
                    stained_material.perceptual_roughness = 0.1;
                }
            }

            if let Some(fire_material_handle) = gltf.named_materials.get("fire".into()) {
                if let Some(mut fire_material) = materials.get_mut(fire_material_handle) {
                    fire_material.alpha_mode = AlphaMode::Add;
                    fire_material.base_color = Color::BLACK;
                    fire_material.reflectance = 0.0;
                    fire_material.emissive = Color::rgb_linear(10.0, 10.0, 10.0);
                    fire_material.emissive_texture = fire_material.base_color_texture.clone();
                }
            }

            if let Some(smoke_material_handle) = gltf.named_materials.get("smoke".into()) {
                if let Some(mut smoke_material) = materials.get_mut(smoke_material_handle) {
                    smoke_material.alpha_mode = AlphaMode::Add;
                    smoke_material.base_color = Color::BLACK;
                    smoke_material.reflectance = 0.0;
                    smoke_material.emissive = Color::rgb(0.5, 0.3, 0.2);
                    smoke_material.emissive_texture = smoke_material.base_color_texture.clone();
                }
            }

            for name in vec![
                "Blue_flower",
                "Fern",
                "Fern1",
                "lambert10",
                "orange_leaf",
                "lambert5",
                "grass",
                "tree_leafs",
                "palm",
                "palm_and_red",
                "Leaf_Floor",
                "lambert8",
                "Pink_flower",
                "lambert11",
            ] {
                if let Some(material_handle) = gltf.named_materials.get(name.into()) {
                    if let Some(mut material) = materials.get_mut(material_handle) {
                        material.alpha_mode = AlphaMode::Mask(0.5);
                    }
                }
            }

            commands.spawn(SceneBundle {
                transform: Transform::from_scale(Vec3::splat(10.0))
                    .with_translation(Vec3::new(-3.0, 0.0, 3.0)),
                scene: gltf.scenes[0].clone(),
                ..default()
            });

            gltf_state.is_loaded = true;
        }
    } else {
        for (name, entity) in &named_entities {
            if name.contains("fire") || name.contains("smoke") {
                commands
                    .entity(entity)
                    .insert((NotShadowCaster, NotShadowReceiver, Patched));
            } else {
                commands.entity(entity).insert(Patched);
            }
        }
    }
}

fn update_camera(mut camera: Query<&mut Transform, With<Camera3d>>, time: Res<Time>) {
    let now = time.elapsed_seconds() * 1.5;
    let mut transform = camera.single_mut();
    let orbit_scale = 5.1 - (now / 10.0).cos() * 4.0;
    *transform = Transform::from_xyz(
        (now / 5.0).cos() * orbit_scale,
        1.8,
        (now / 5.0).sin() * orbit_scale,
    )
    .looking_at(
        Vec3::new(
            ((now + 2.3) / 5.0).sin() * orbit_scale * 0.1,
            orbit_scale * 0.1,
            ((now + 2.3) / 5.0).cos() * orbit_scale * 0.1,
        ),
        Vec3::Y,
    );

    transform.rotate(Quat::from_rotation_z(orbit_scale * 0.01));
}
