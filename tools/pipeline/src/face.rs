use anyhow::{anyhow, Result};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use rustface::{Detector, ImageData};

pub const MIN_BOX_WIDTH: u32 = 200;
pub const CROP_FACTOR: f64 = 2.2;
pub const OUT_SIDE: u32 = 512;
pub const JPEG_QUALITY: u8 = 82;

static MODEL: &[u8] = include_bytes!("../model/seeta_fd_frontal_v1.0.bin");

pub struct Face {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub enum Outcome {
    One(Face),
    None,
    Many(usize),
    Small(u32),
}

pub struct FaceDetector {
    inner: Box<dyn Detector>,
}

impl FaceDetector {
    pub fn new() -> Result<Self> {
        let model = rustface::model::read_model(MODEL)?;
        let mut inner = rustface::create_detector_with_model(model);
        inner.set_min_face_size(40);
        inner.set_score_thresh(2.0);
        inner.set_pyramid_scale_factor(0.8);
        inner.set_slide_window_step(4, 4);
        Ok(Self { inner })
    }

    pub fn detect(&mut self, img: &DynamicImage) -> Outcome {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Outcome::None;
        }
        let gray = img.to_luma8();
        let data = ImageData::new(gray.as_raw(), w, h);
        let faces = self.inner.detect(&data);
        match faces.len() {
            0 => Outcome::None,
            1 => {
                let b = faces[0].bbox();
                if b.width() < MIN_BOX_WIDTH {
                    Outcome::Small(b.width())
                } else {
                    Outcome::One(Face {
                        x: b.x(),
                        y: b.y(),
                        w: b.width(),
                        h: b.height(),
                    })
                }
            }
            n => Outcome::Many(n),
        }
    }
}

pub struct Square {
    pub x: u32,
    pub y: u32,
    pub side: u32,
}

pub fn crop_square(img_w: u32, img_h: u32, face: &Face) -> Square {
    let side = (CROP_FACTOR * face.w as f64)
        .round()
        .min(img_w as f64)
        .min(img_h as f64);
    let cx = face.x as f64 + face.w as f64 / 2.0;
    let cy = face.y as f64 + face.h as f64 / 2.0;
    let x = (cx - side / 2.0).round().clamp(0.0, img_w as f64 - side);
    let y = (cy - side / 2.0).round().clamp(0.0, img_h as f64 - side);
    Square {
        x: x as u32,
        y: y as u32,
        side: side as u32,
    }
}

pub fn crop(img: &DynamicImage, face: &Face) -> DynamicImage {
    let (w, h) = img.dimensions();
    let sq = crop_square(w, h, face);
    img.crop_imm(sq.x, sq.y, sq.side, sq.side).resize_exact(
        OUT_SIDE,
        OUT_SIDE,
        FilterType::Lanczos3,
    )
}

pub fn encode_jpeg(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    enc.encode_image(&img.to_rgb8())
        .map_err(|e| anyhow!("jpeg encode: {e}"))?;
    Ok(out)
}

pub fn decode(content_type: &str, bytes: &[u8]) -> Option<DynamicImage> {
    let format = match content_type.split(';').next()?.trim() {
        "image/jpeg" | "image/jpg" => image::ImageFormat::Jpeg,
        "image/png" => image::ImageFormat::Png,
        "image/tiff" => image::ImageFormat::Tiff,
        "image/gif" => image::ImageFormat::Gif,
        "image/webp" => image::ImageFormat::WebP,
        _ => return None,
    };
    image::load_from_memory_with_format(bytes, format).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(x: i32, y: i32, w: u32, h: u32) -> Face {
        Face { x, y, w, h }
    }

    #[test]
    fn square_is_2_2_times_box_width_centred_on_box() {
        let sq = crop_square(2000, 2000, &face(800, 600, 300, 300));
        assert_eq!(sq.side, 660);
        assert_eq!(sq.x, 950 - 330);
        assert_eq!(sq.y, 750 - 330);
    }

    #[test]
    fn square_clamps_at_top_left() {
        let sq = crop_square(2000, 2000, &face(10, 20, 300, 300));
        assert_eq!(sq.side, 660);
        assert_eq!((sq.x, sq.y), (0, 0));
    }

    #[test]
    fn square_clamps_at_bottom_right() {
        let sq = crop_square(1000, 900, &face(650, 550, 300, 300));
        assert_eq!(sq.side, 660);
        assert_eq!((sq.x, sq.y), (340, 240));
    }

    #[test]
    fn image_smaller_than_square_takes_largest_square() {
        let sq = crop_square(500, 700, &face(100, 50, 300, 300));
        assert_eq!(sq.side, 500);
        assert_eq!(sq.x, 0);
        assert_eq!(sq.y, 0);
        let sq = crop_square(500, 700, &face(100, 350, 300, 300));
        assert_eq!(sq.y, 200);
    }

    #[test]
    fn decode_picks_format_from_content_type() {
        let img = DynamicImage::new_rgb8(4, 4);
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, image::ImageFormat::Png).unwrap();
        assert!(decode("image/png", png.get_ref()).is_some());
        assert!(decode("image/jpeg", png.get_ref()).is_none());
        assert!(decode("image/svg+xml", b"<svg/>").is_none());
    }
}
