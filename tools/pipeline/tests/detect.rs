use image::GenericImageView;
use pipeline::face::{self, FaceDetector, Outcome, MIN_BOX_WIDTH, OUT_SIDE};
use std::path::Path;

#[test]
fn turing_portrait_yields_one_face_and_a_512_crop() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Q7251.jpg");
    let img = image::open(&path).expect("fixture decodes");
    let mut det = FaceDetector::new().expect("model loads");
    let face = match det.detect(&img) {
        Outcome::One(f) => f,
        Outcome::None => panic!("no face found"),
        Outcome::Many(n) => panic!("{n} faces found"),
        Outcome::Small(w) => panic!("face only {w} px wide"),
    };
    assert!(face.w >= MIN_BOX_WIDTH, "face {} px wide", face.w);
    let crop = face::crop(&img, &face);
    assert_eq!(crop.dimensions(), (OUT_SIDE, OUT_SIDE));
    let jpeg = face::encode_jpeg(&crop).unwrap();
    let back = image::load_from_memory(&jpeg).unwrap();
    assert_eq!(back.dimensions(), (OUT_SIDE, OUT_SIDE));
}
