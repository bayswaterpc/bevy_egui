use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy::{
    prelude::*,
    core::FixedTimestep,
    render::pass::ClearColor,
    window::{CreateWindow, WindowDescriptor, WindowId},
    sprite::collide_aabb::{collide, Collision},
};
use bevy::ecs::schedule::ShouldRun;
use bevy::app::AppExit;

// NOTE: this "state based" approach to multiple windows is a short term workaround.
// Future Bevy releases shouldn't require such a strict order of operations.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum AppState {
    CentralPanelState,
    GameState,
}


/// An implementation of the classic game "Breakout" with egui panels
const TIME_STEP: f32 = 1.0 / 60.0;
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(Color::rgb(0.9, 0.9, 0.9)))
        .add_state(AppState::CentralPanelState)
        .add_startup_system(setup.system())
        .add_system_set( //follows timestep & state requirements
            SystemSet::new()
                .with_run_criteria(
                    FixedTimestep::step(TIME_STEP as f64).chain(
                        (|In(input): In<ShouldRun>, state: Res<State<AppState>>| {
                            if state.current() == &AppState::GameState {
                                input
                            } else {
                                ShouldRun::No
                            }
                        })
                            .system(),
                    ),
                )
                // Wait for game to load
                .with_system(paddle_movement_system.system())
                .with_system(ball_collision_system.system())
                .with_system(ball_movement_system.system()),

        )
        .add_system_set(
            SystemSet::on_update(AppState::GameState)
                .with_system(scoreboard_system.system())
        )
        .add_system(ui_egui.system())
        .run();
}

struct Paddle {
    speed: f32,
}

struct Ball {
    velocity: Vec3,
}

struct Scoreboard {
    score: usize,
}

enum Collider {
    Solid,
    Scorable,
    Paddle,
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Add the game's entities to our world

    // cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    // paddle
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            transform: Transform::from_xyz(0.0, -215.0, 0.0),
            sprite: Sprite::new(Vec2::new(120.0, 30.0)),
            ..Default::default()
        })
        .insert(Paddle { speed: 500.0 })
        .insert(Collider::Paddle);
    // ball
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
            transform: Transform::from_xyz(0.0, -50.0, 1.0),
            sprite: Sprite::new(Vec2::new(30.0, 30.0)),
            ..Default::default()
        })
        .insert(Ball {
            velocity: 400.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
        });
    // scoreboard
    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![
                TextSection {
                    value: "Score: ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.5, 0.5, 1.0),
                    },
                },
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(1.0, 0.5, 0.5),
                    },
                },
            ],
            ..Default::default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });

    // Add walls
    let wall_material = materials.add(Color::rgb(0.8, 0.8, 0.8).into());
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    // left
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(-bounds.x / 2.0, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y + wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // right
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(bounds.x / 2.0, 0.0, 0.0),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y + wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // bottom
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_xyz(0.0, -bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);
    // top
    commands
        .spawn_bundle(SpriteBundle {
            material: wall_material,
            transform: Transform::from_xyz(0.0, bounds.y / 2.0, 0.0),
            sprite: Sprite::new(Vec2::new(bounds.x + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .insert(Collider::Solid);

    // Add bricks
    let brick_rows = 4;
    let brick_columns = 5;
    let brick_spacing = 20.0;
    let brick_size = Vec2::new(150.0, 30.0);
    let bricks_width = brick_columns as f32 * (brick_size.x + brick_spacing) - brick_spacing;
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - brick_size.x) / 2.0, 100.0, 0.0);
    let brick_material = materials.add(Color::rgb(0.5, 0.5, 1.0).into());
    for row in 0..brick_rows {
        let y_position = row as f32 * (brick_size.y + brick_spacing);
        for column in 0..brick_columns {
            let brick_position = Vec3::new(
                column as f32 * (brick_size.x + brick_spacing),
                y_position,
                0.0,
            ) + bricks_offset;
            // brick
            commands
                .spawn_bundle(SpriteBundle {
                    material: brick_material.clone(),
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .insert(Collider::Scorable);
        }
    }
}

fn paddle_movement_system(
    mut app_state: ResMut<State<AppState>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform)>,
) {
    match app_state.current().clone() {
        AppState::GameState => {},
        _ => return,
    }
    if let Ok((paddle, mut transform)) = query.single_mut() {
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Left) {
            direction -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::Right) {
            direction += 1.0;
        }

        let translation = &mut transform.translation;
        // move the paddle horizontally
        translation.x += direction * paddle.speed * TIME_STEP;
        // bound the paddle within the walls
        translation.x = translation.x.min(380.0).max(-380.0);
    }
}

fn ball_movement_system(mut app_state: ResMut<State<AppState>>, mut ball_query: Query<(&Ball, &mut Transform)>) {
    match app_state.current().clone() {
        AppState::GameState => {},
        _ => {
            println!("Used to Always Hit");
            return;
        },
    }
    if let Ok((ball, mut transform)) = ball_query.single_mut() {
        transform.translation += ball.velocity * TIME_STEP;
    }
}

fn scoreboard_system(mut app_state: ResMut<State<AppState>>, scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    match app_state.current().clone() {
        AppState::GameState => {
            println!("Hits Correctly");
        },
        _ => {
            println!("Never hits");
            return;
        },
    }
    let mut text = query.single_mut().unwrap();
    text.sections[0].value = format!("Score: {}", scoreboard.score);
}

fn ball_collision_system(
    mut app_state: ResMut<State<AppState>>,
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    collider_query: Query<(Entity, &Collider, &Transform, &Sprite)>,
) {
    match app_state.current().clone() {
        AppState::GameState => {},
        _ => return,
    }
    if let Ok((mut ball, ball_transform, sprite)) = ball_query.single_mut() {
        let ball_size = sprite.size;
        let velocity = &mut ball.velocity;

        // check collision with walls
        for (collider_entity, collider, transform, sprite) in collider_query.iter() {
            let collision = collide(
                ball_transform.translation,
                ball_size,
                transform.translation,
                sprite.size,
            );
            if let Some(collision) = collision {
                // scorable colliders should be despawned and increment the scoreboard on collision
                if let Collider::Scorable = *collider {
                    scoreboard.score += 1;
                    commands.entity(collider_entity).despawn();
                }

                // reflect the ball when it collides
                let mut reflect_x = false;
                let mut reflect_y = false;

                // only reflect if the ball's velocity is going in the opposite direction of the
                // collision
                match collision {
                    Collision::Left => reflect_x = velocity.x > 0.0,
                    Collision::Right => reflect_x = velocity.x < 0.0,
                    Collision::Top => reflect_y = velocity.y < 0.0,
                    Collision::Bottom => reflect_y = velocity.y > 0.0,
                }

                // reflect velocity on the x-axis if we hit something on the x-axis
                if reflect_x {
                    velocity.x = -velocity.x;
                }

                // reflect velocity on the y-axis if we hit something on the y-axis
                if reflect_y {
                    velocity.y = -velocity.y;
                }

                // break if this collide is on a solid, otherwise continue check whether a solid is
                // also in collision
                if let Collider::Solid = *collider {
                    break;
                }
            }
        }
    }
}


fn ui_egui(
    mut app_state: ResMut<State<AppState>>,
    egui_context: Res<EguiContext>,
) {
    egui::SidePanel::left("side_panel", 200.0).show(egui_context.ctx(), |ui| {
        ui.heading("Side Panel");
        if ui.button("Central Panel").clicked() {
            match app_state.set(AppState::CentralPanelState) {
                Ok(_) => {},
                Err(e) => {println!("{:?}", e)},
            }
        }
        if ui.button("Game Panel").clicked() {
            match app_state.set(AppState::GameState) {
                Ok(_) => {},
                Err(e) => {println!("{:?}", e)},
            }
        }
        if ui.button("Console Log").clicked() {
            println!("app_state {:?}", app_state.current());
        }
    });

    match app_state.current().clone() {
        AppState::CentralPanelState => {
            egui_center_panel(app_state, egui_context);
        },
        _ => return,
    }

}

fn egui_center_panel(
    mut app_state: ResMut<State<AppState>>,
    egui_context: Res<EguiContext>,
) {

    egui::CentralPanel::default().show(egui_context.ctx(), |ui| {
        ui.heading("Central Panel");
        if ui.button("Console Log").clicked() {
            println!("app_state {:?}", app_state.current());
        }
    });


}
