use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use pipeline::face::{self, FaceDetector, Outcome, MIN_BOX_WIDTH, OUT_SIDE};
use std::path::Path;

fn turing() -> DynamicImage {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Q7251.jpg");
    image::open(&path).expect("fixture decodes")
}

#[test]
fn turing_portrait_yields_one_face_and_a_512_crop() {
    let img = turing();
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

#[test]
fn blank_image_yields_no_face() {
    let mut det = FaceDetector::new().expect("model loads");
    assert!(matches!(
        det.detect(&DynamicImage::new_rgb8(600, 600)),
        Outcome::None
    ));
}

#[test]
fn two_portraits_side_by_side_yield_many() {
    let half = turing().resize(356, 2000, FilterType::Triangle);
    let (w, h) = half.dimensions();
    let mut pair = DynamicImage::new_rgb8(w * 2, h);
    image::imageops::replace(&mut pair, &half, 0, 0);
    image::imageops::replace(&mut pair, &half, i64::from(w), 0);
    let mut det = FaceDetector::new().expect("model loads");
    match det.detect(&pair) {
        Outcome::Many(2) => {}
        Outcome::Many(n) => panic!("{n} faces found"),
        Outcome::One(_) => panic!("one face found"),
        Outcome::None => panic!("no face found"),
        Outcome::Small(w) => panic!("face only {w} px wide"),
    }
}

#[test]
fn downscaled_portrait_yields_small() {
    let small = turing().resize(300, 2000, FilterType::Triangle);
    let mut det = FaceDetector::new().expect("model loads");
    match det.detect(&small) {
        Outcome::Small(w) => assert!(w < MIN_BOX_WIDTH, "face {w} px wide"),
        Outcome::One(f) => panic!("face {} px wide passed", f.w),
        Outcome::None => panic!("no face found"),
        Outcome::Many(n) => panic!("{n} faces found"),
    }
}
