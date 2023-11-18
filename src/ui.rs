use bevy::{prelude::*, winit::WinitSettings};
use leafwing_input_manager::prelude::*;

pub struct UI;

impl Plugin for UI {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            // .add_startup_system_to_stage(StartupStage::PostStartup, spawn_ui)
        ;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    //root node (left panel)
    commands.spawn(NodeBundle{
        style: Style {
            size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        background_color: Color::rgb(0.65, 0.65, 0.65).into(),
        ..default()
    }).with_children(|parent|{
        // left vertical fill (content)
        parent
            .spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    ..default()
                },
                background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                ..default()
            })
            .with_children(|parent| {
                // text
                parent.spawn(
                    TextBundle::from_section(
                        "Text Example",
                        TextStyle {
                            font: default(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(5.0)),
                        ..default()
                    }),
                );
            });
    });
}