use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct ControlledCamera;

impl Plugin for ControlledCamera {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<CameraMovement>::default())
            .add_startup_system(setup)
            .add_system(zoom_camera)
            .add_system(pan_camera.after(zoom_camera));
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(InputManagerBundle::<CameraMovement> {
            input_map: InputMap::default()
                .insert(SingleAxis::mouse_wheel_y(), CameraMovement::Zoom)
                .insert(DualAxis::mouse_motion(), CameraMovement::Pan)
                .insert(MouseButton::Left, CameraMovement::Clicked)
                .build(),
            ..default()
        });

}

#[derive(Actionlike, Clone, Debug, Copy, PartialEq, Eq)]
enum CameraMovement {
    Zoom,
    Pan,
    Clicked,
}

fn zoom_camera(
    mut query: Query<(&mut OrthographicProjection, &ActionState<CameraMovement>), With<Camera2d>>,
) {
    const CAMERA_ZOOM_RATE: f32 = 0.05;

    let (mut camera_projection, action_state) = query.single_mut();
    // Here, we use the `action_value` method to extract the total net amount that the mouse wheel has travelled
    // Up and right axis movements are always positive by default
    let zoom_delta = action_state.value(CameraMovement::Zoom);

    // We want to zoom in when we use mouse wheel up
    // so we increase the scale proportionally
    // Note that the projections scale should always be positive (or our images will flip)
    camera_projection.scale *= 1. - zoom_delta * CAMERA_ZOOM_RATE;
}

fn pan_camera(mut query: Query<(&mut Transform, &OrthographicProjection, &ActionState<CameraMovement>), With<Camera2d>>) {
    const CAMERA_PAN_RATE: f32 = 1.;

    let (mut camera_transform, projection, action_state) = query.single_mut();

    let camera_pan_vector = action_state.axis_pair(CameraMovement::Pan).unwrap();

    // Because we're moving the camera, not the object, we want to pan in the opposite direction
    // However, UI cordinates are inverted on the y-axis, so we need to flip y a second time
    if action_state.pressed(CameraMovement::Clicked) {
        camera_transform.translation.x -= CAMERA_PAN_RATE * projection.scale * camera_pan_vector.x();
        camera_transform.translation.y += CAMERA_PAN_RATE * projection.scale * camera_pan_vector.y();
    }
}