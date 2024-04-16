use bevy::{
    prelude::*, render::camera::ScalingMode,
    sprite::MaterialMesh2dBundle,
};
use bevy_ecs_tilemap::prelude::*;
use bevy_xpbd_2d::{math::Vector, prelude::*};

fn main() {
    App::new()
        .insert_resource(Gravity(Vector::ZERO))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from(
                            "Infinite Pong",
                        ),
                        ..Default::default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            TilemapPlugin,
            PhysicsPlugins::default(),
            // PhysicsDebugPlugin::default(),
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, ball_collision)
        .run();
}

#[derive(Component)]
struct Ball;

#[derive(PhysicsLayer)]
enum Layer {
    Player1,
    Player2,
    All,
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let map_size = TilemapSize { x: 32, y: 18 };
    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.spawn(Camera2dBundle {
        camera: Camera {
            // hdr: todo!(),
            ..default()
        },
        projection: OrthographicProjection {
            far: 1000.,
            near: -1000.,
            scaling_mode: ScalingMode::Fixed {
                width: map_size.x as f32 * tile_size.x,
                height: map_size.y as f32 * tile_size.y,
            },
            ..default()
        },
        ..default()
    });

    let texture_handle: Handle<Image> =
        asset_server.load("tiles-no-border.png");

    // Create a tilemap entity a little early.
    // We want this entity early because we need to tell each tile which tilemap entity
    // it is associated with. This is done with the TilemapId component on each tile.
    // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
    // will contain various necessary components, such as `TileStorage`.
    let tilemap_entity = commands.spawn_empty().id();

    // To begin creating the map we will need a `TileStorage` component.
    // This component is a grid of tile entities and is used to help keep track of individual
    // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
    // per layer, each with their own `TileStorage` component.
    let mut tile_storage = TileStorage::empty(map_size);

    commands.entity(tilemap_entity).with_children(
        |builder| {
            // Spawn the elements of the tilemap.
            // Alternatively, you can use helpers::filling::fill_tilemap.
            for x in 0..map_size.x {
                for y in 0..map_size.y {
                    let tile_pos = TilePos { x, y };
                    let center = tile_pos.center_in_world(
                        &grid_size, &map_type,
                    );

                    let tile_entity = builder
                        .spawn((
                            TileBundle {
                                position: tile_pos,
                                tilemap_id: TilemapId(
                                    tilemap_entity,
                                ),
                                texture_index: if x
                                    < (map_size.x / 2)
                                {
                                    TileTextureIndex(0)
                                } else {
                                    TileTextureIndex(1)
                                },
                                ..Default::default()
                            },
                            // In order to add colliders, we need the world position of the tile as a Vec2.
                            Transform::from_xyz(
                                center.x, center.y, 0.,
                            ),
                            Collider::rectangle(
                                tile_size.x,
                                tile_size.y,
                            ),
                            RigidBody::Static,
                            if x < (map_size.x / 2) {
                                CollisionLayers::new(
                                    [Layer::Player1],
                                    [Layer::Player2],
                                )
                            } else {
                                CollisionLayers::new(
                                    [Layer::Player2],
                                    [Layer::Player1],
                                )
                            },
                        ))
                        .id();
                    tile_storage
                        .set(&tile_pos, tile_entity);
                }
            }
        },
    );

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(
            &map_size, &grid_size, &map_type, 0.0,
        ),
        ..Default::default()
    });

    let player_ball_radius = 7.5;
    let player_ball_mesh =
        meshes.add(Circle::new(player_ball_radius));

    let blue_material =
        materials.add(Color::rgb(0.36, 0.43, 0.88));
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: player_ball_mesh.clone().into(),
            material: blue_material.clone(),
            transform: Transform::from_xyz(
                -tile_size.x * (map_size.x / 4) as f32,
                0.,
                10.0,
            ),
            ..default()
        },
        RigidBody::Dynamic,
        Collider::circle(player_ball_radius),
        LinearVelocity(Vec2::new(-200., -200.)),
        CollisionLayers::new(
            [Layer::Player1],
            [Layer::Player2, Layer::All],
        ),
        Restitution::new(1.)
            .with_combine_rule(CoefficientCombine::Max),
        Friction::new(0.0)
            .with_combine_rule(CoefficientCombine::Min),
        Ball,
    ));

    let green_material =
        materials.add(Color::rgb(0.6, 0.9, 0.31));
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: player_ball_mesh.clone().into(),
            material: green_material.clone(),
            transform: Transform::from_xyz(
                tile_size.x * (map_size.x / 4) as f32,
                0.,
                10.0,
            ),
            ..default()
        },
        RigidBody::Dynamic,
        Collider::circle(player_ball_radius),
        LinearVelocity(Vec2::new(200., 200.)),
        CollisionLayers::new(
            [Layer::Player2],
            [Layer::Player1, Layer::All],
        ),
        Restitution::new(1.)
            .with_combine_rule(CoefficientCombine::Max),
        Friction::new(0.0)
            .with_combine_rule(CoefficientCombine::Min),
        Ball,
    ));

    let size = Vec2::new(
        map_size.x as f32 * tile_size.x,
        map_size.y as f32 * tile_size.y,
    ) / 2.;
    let vertices = vec![
        Vector::new(-size.x, -size.y),
        Vector::new(-size.x, size.y),
        Vector::new(size.x, size.y),
        Vector::new(size.x, -size.y),
    ];
    let indices = vec![[0, 1], [1, 2], [2, 3], [3, 0]];
    commands.spawn((
        SpatialBundle::default(),
        RigidBody::Static,
        Collider::polyline(vertices, Some(indices)),
        CollisionLayers::new(
            [Layer::All],
            [Layer::Player1, Layer::Player2],
        ),
    ));
}

fn ball_collision(
    balls: Query<&CollidingEntities, With<Ball>>,
    mut tiles: Query<(
        &mut TileTextureIndex,
        &mut CollisionLayers,
    )>,
) {
    for colliding_entities in &balls {
        if colliding_entities.is_empty() {
            continue;
        }
        for entity in colliding_entities.iter() {
            if let Ok((mut texture_index, mut layers)) =
                tiles.get_mut(*entity)
            {
                match texture_index.0 {
                    0 => {
                        *texture_index =
                            TileTextureIndex(1);
                        *layers = CollisionLayers::new(
                            [Layer::Player2],
                            [Layer::Player1],
                        );
                    }
                    1 => {
                        *texture_index =
                            TileTextureIndex(0);
                        *layers = CollisionLayers::new(
                            [Layer::Player1],
                            [Layer::Player2],
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}
