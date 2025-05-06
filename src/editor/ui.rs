use bevy::{color::palettes::css, platform::collections::HashSet, prelude::*, ui::BackgroundColor};

use crate::core::{
    media::{surface::Surface, MediaCollection},
    AppState,
};

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

#[derive(Component)]
pub struct SurfaceList;

fn init_ui(mut commands: Commands) {
    commands.spawn((
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
        SurfaceList,
    ));
}

fn update_surf_list(
    surfaces: Res<MediaCollection<Surface>>,
    q_lists: Query<Entity, With<SurfaceList>>,
    mut commands: Commands,
) {
    for list_entity in q_lists.iter() {
        let mut entity_cmds = commands.entity(list_entity);
        entity_cmds.despawn_related::<Children>();
        entity_cmds.with_children(|builder| {
            for (_id, surface) in surfaces.iter() {
                builder
                    .spawn((
                        Node {
                            padding: UiRect {
                                left: Val::Px(8.),
                                right: Val::Px(8.),
                                top: Val::Px(8.),
                                bottom: Val::Px(8.),
                            },
                            ..default()
                        },
                        Button,
                    ))
                    .with_child(Text::new(surface.meta.path.to_string_lossy()));
            }
        });
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<ClickBlocker>();
    app.add_systems(OnEnter(AppState::InEditor), init_ui);
    app.add_systems(
        Update,
        (
            clicktest,
            update_surf_list.run_if(resource_exists_and_changed::<MediaCollection<Surface>>),
        ),
    );
}
