//! The camera against the game: `tools/test_stream_rs.py cameras` ran
//! `ccCam::SetMatrix_PosRotXYZ` and `ccView::SetView` in eemu after
//! `SetFrame`, for the desktop's camera and 300 seeded ones, and kept the
//! camera's matrix and the view's `world_screen`. This decodes the same
//! records into one camera, in order, and compares both bit for bit.

use piney_desktop::camera::{Camera, frame_projection};
use piney_desktop::view::{Frame, View};

const FIXTURE: &str = include_str!("camera_fixture.txt");

#[test]
fn cameras_place_and_project_as_the_game_s() {
    let mut cam = Camera::default();
    let mut n = 0;
    for line in FIXTURE.lines().filter(|l| l.starts_with("camera ")) {
        let w: Vec<u32> = line.split(' ').skip(1).map(|x| u32::from_str_radix(x, 16).unwrap()).collect();
        // The record's values: position, degrees, the unused one, the fov.
        let rec = [w[1], w[2], w[3], w[4], w[5], w[6], 0, w[7]];
        let f = |i: usize| f32::from_bits(w[8 + i]);
        let frame = Frame { x: f(0), y: f(1), w: f(2), h: f(3), cx: f(4), cy: f(5), ax: f(6), ay: f(7) };
        cam = cam.decode(w[0], &rec);
        let view = View::new(frame_projection(&frame), cam);
        assert_eq!(cam.matrix.concat(), w[16..32], "camera {n}: matrix");
        assert_eq!(view.world_screen_bits().concat(), w[32..48], "camera {n}: world_screen");
        n += 1;
    }
    assert_eq!(n, 301);
}
