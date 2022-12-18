use bevy::{
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{mesh::Indices, primitives},
    time::FixedTimestep,
};

use rand::Rng;

const TIME_STEP: f32 = 0.01666666;
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        window: WindowDescriptor {
            monitor: MonitorSelection::Index(1),
            ..default()
        },
        ..default()
    }))
    .add_plugin(WireframePlugin)
    .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
    .add_startup_system(setup)
    .add_system(apply_velocity)
    .add_system(bevy::window::close_on_esc)
    .add_system_set(
        SystemSet::new()
            .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
            .with_system(apply_velocity),
    )
    .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec3);

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-10., 5., 0.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // ambient_light.color = Color::WHITE;
    // ambient_light.brightness = 1.0;

    {
        let material = materials.add(Color::ORANGE.into());
        let mesh = meshes.add(Mesh::from(shape::Plane { size: 500. }));

        commands.spawn(PbrBundle {
            material,
            mesh,
            transform: Transform::from_xyz(0., -50., 0.),
            ..default()
        });
    }

    {
        shape::Plane { ..default() };
        let material = materials.add(StandardMaterial {
            // emissive: Color::AQUAMARINE,
            // double_sided: true,
            // cull_mode: None,
            // base_color: Color::AQUAMARINE,

            // base_color: Color::AQUAMARINE,
            ..default()
        });

        let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);

        const MESH_WIDTH: u32 = 200;
        const MESH_DEPTH: u32 = 20;

        const CELL_SIZE: f32 = 1.2;
        let (positions, indicies) = generate_terrain_chunk(MESH_WIDTH, MESH_DEPTH, CELL_SIZE);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_indices(Some(indicies));

        let mesh_handle = meshes.add(mesh);

        // Pos x towards the camera
        // Pos z to the right

        commands.spawn((
            PbrBundle {
                mesh: mesh_handle,
                material,
                transform: Transform::from_xyz(0., 0., -(MESH_DEPTH as f32) / 2.),
                ..default()
            },
            Velocity(Vec3::new(-10.5, 0., 0.)),
            Wireframe,
        ));
    }
}

fn generate_terrain_chunk(width: u32, depth: u32, cell_size: f32) -> (Vec<[f32; 3]>, Indices) {
    let mut verts = vec![];
    let mut indicies: Vec<u32> = vec![];

    let mut rng = rand::thread_rng();

    for x in 0..width {
        for z in 0..depth {
            verts.push([
                x as f32 * cell_size,
                rng.gen_range(0f32..2.),
                z as f32 * cell_size,
            ]);
        }
    }

    println!("{:#?}", verts);

    // for row in 0..depth - 1 {

    // }

    for x in 0..width - 1 {
        for z in 0..depth - 1 {
            let i = x * depth + z;
            indicies.extend_from_slice(&[i, i + 1, i + depth, i + depth, i + 1, i + depth + 1]);
        }
    }

    // indicies.push(0);
    // indicies.push(1);
    // indicies.push(2);

    // indicies.push(1);
    // indicies.push(3);
    // indicies.push(2);
    // indicies.push(2);
    // indicies.push(0);
    // indicies.push(3);

    (verts, Indices::U32(indicies))
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, vel) in query.iter_mut() {
        transform.translation += vel.0 * TIME_STEP;
    }
}
