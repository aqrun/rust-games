//! 贪吃蛇游戏
//! https://mbuffett.com/posts/bevy-snake-tutorial/
//!
use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer, window::PrimaryWindow};
use rand::prelude::*;

/// 蛇头颜色
const SNAKE_HEAD_COLOR: Color = Color::linear_rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
/// 食物颜色
const FOOD_COLOR: Color = Color::srgb(1.0, 0.0, 1.0);
/// 蛇一段的宽高
const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

/// snakeHead空结构体组件
#[derive(Component)]
struct SnakeHead {
    direction: Directions,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Resource, Default)]
struct SnakeSegments(Vec<Entity>);

#[derive(Resource, Default)]
struct LastTailPosition(Option<Position>);

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Directions {
    Left,
    Up,
    Right,
    Down,
}

impl Directions {
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

/// 定义生长事件
#[derive(Event)]
struct GrowthEvent;

#[derive(Event)]
struct GameOverEvent;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("贪吃蛇"),
                resolution: [500., 500.].into(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, spawn_snake)
        .add_systems(
            FixedUpdate,
            (
                snake_movement,
                snake_eating.after(snake_movement),
                snake_growth.after(snake_eating),
            ).run_if(on_timer(Duration::from_millis(500))),
        )
        .add_systems(
            FixedUpdate,
            food_spawner.run_if(on_timer(Duration::from_secs(2))),
        )
        .add_systems(Update, snake_movement_input.before(snake_movement))
        .add_systems(Update, game_over.after(snake_movement))
        .add_systems(PostUpdate, (position_translation, size_scaling))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// 生成蛇头
fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    let head = commands.spawn((
        // 指定蛇头颜色
        Sprite {
            color: SNAKE_HEAD_COLOR,
            ..default()
        },
        Transform::from_scale(Vec3::new(10.0, 10.0, 10.0)),
        // 绑定蛇头组件
        SnakeHead {
            direction: Directions::Up,
        },
        Position { x: 3, y: 3 },
        Size::square(0.8),
    )).id();
    let tail = spawn_segment(commands, Position { x: 3, y: 2});

    *segments = SnakeSegments(vec![head, tail]);
}

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
    commands.spawn((
        Sprite {
            color: SNAKE_SEGMENT_COLOR,
            ..default()
        },
        Transform::from_scale(Vec3::new(10., 10., 10.)),
        SnakeSegment,
        position,
        Size::square(0.65)
    )).id()
}

fn food_spawner(mut commands: Commands, time: Res<Time>) {
    let mut rng = rand::rng();

    commands.spawn((
        Sprite {
            color: FOOD_COLOR,
            ..default()
        },
        Transform::from_scale(Vec3::new(10.0, 10.0, 10.0)),
        Food,
        Position {
            x: (rng.random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (rng.random::<f32>() * ARENA_HEIGHT as f32) as i32,
        },
        Size::square(0.8),
    ));
}

fn size_scaling(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&Size, &mut Transform)>,
) {
    let window = windows.get_single().unwrap();

    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_translation(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&Position, &mut Transform)>,
) {
    let window = windows.get_single().unwrap();
    let w = window.width() as f32;
    let h = window.height() as f32;

    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.0) + (tile_size / 2.0)
    }

    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, w, ARENA_WIDTH as f32),
            convert(pos.y as f32, h, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut positions: Query<&mut Position>,
    mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if let Some((mut head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments.0
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos  = positions.get_mut(head_entity).unwrap();
        match &head.direction {
            Directions::Left => {
                head_pos.x -= 1;
            }
            Directions::Right => {
                head_pos.x += 1;
            }
            Directions::Up => {
                head_pos.y += 1;
            }
            Directions::Down => {
                head_pos.y -= 1;
            }
        };

        if head_pos.x < 0
            || head_pos.y < 0
            || head_pos.x as u32 >= ARENA_WIDTH
            || head_pos.y as u32 >= ARENA_HEIGHT
        {
            game_over_writer.send(GameOverEvent);
        }

        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }

        segment_positions
            .iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });
        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
    }
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Directions::Left
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Directions::Down
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Directions::Up
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Directions::Right
        } else {
            head.direction
        };

        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        segments.0.push(spawn_segment(commands, last_tail_position.0.unwrap()));
    }
}

fn game_over (
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
) {
    if reader.read().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, segments_res);
    }
}
