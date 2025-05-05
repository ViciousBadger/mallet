use bevy::{color::palettes::css, prelude::*, ui::BackgroundColor, utils::HashSet};

use crate::core::AppState;

#[derive(Component)]
pub struct PreventClicks;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct ClickBlocker {
    entities_blocking: HashSet<Entity>,
}

impl ClickBlocker {
    pub fn can_click(&self) -> bool {
        self.entities_blocking.is_empty()
    }
}

#[allow(clippy::type_complexity)]
fn clicktest(
    q_changed_inter: Query<(Entity, &Interaction), (With<PreventClicks>, Changed<Interaction>)>,
    mut click_blocker: ResMut<ClickBlocker>,
) {
    for (entity, inter) in &q_changed_inter {
        match inter {
            Interaction::Pressed | Interaction::Hovered => click_blocker.insert(entity),
            Interaction::None => click_blocker.remove(&entity),
        };
    }
}

fn init_ui(mut commands: Commands) {
    commands
        .spawn((
            PreventClicks,
            StateScoped(AppState::InEditor),
            Interaction::default(),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Px(300.0),
                height: Val::Percent(100.0),
                padding: UiRect {
                    left: Val::Px(16.),
                    right: Val::Px(16.),
                    top: Val::Px(16.),
                    bottom: Val::Px(16.),
                },
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                ..default()
            },
            BackgroundColor(Color::Srgba(css::BLACK)),
        ))
        .with_children(|builder| {
            builder.spawn((
                Node {
                    padding: UiRect {
                        left: Val::Px(8.),
                        right: Val::Px(8.),
                        top: Val::Px(8.),
                        bottom: Val::Px(8.),
                    },
                    ..default()
                }, // font?
                Text::new("hello world!!!!"),
            ));
            builder.spawn((
                Node {
                    padding: UiRect {
                        left: Val::Px(8.),
                        right: Val::Px(8.),
                        top: Val::Px(8.),
                        bottom: Val::Px(8.),
                    },
                    ..default()
                }, // font?
                Text::new("hello world!!!!"),
            ));
        });
}

pub fn plugin(app: &mut App) {
    app.init_resource::<ClickBlocker>();
    app.add_systems(OnEnter(AppState::InEditor), init_ui);
    app.add_systems(Update, clicktest);
}
