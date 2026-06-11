//! Smoke tests for VBV rate capping: building a constrained-quality
//! (CRF + VBV) encoder and adjusting the cap at runtime through
//! `Encoder::reconfig_vbv` without rebuilding the encoder.

extern crate x264;

use x264::{Colorspace, Image, Plane, Preset, Setup, Tune};

const WIDTH: i32 = 320;
const HEIGHT: i32 = 240;

/// Builds a deterministic pseudo-random I420 frame so the encoder has
/// non-trivial content to compress on every call.
fn synthetic_frame(seed: u32) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (w, h) = (WIDTH as usize, HEIGHT as usize);
    let mut state = seed.wrapping_mul(2654435761).wrapping_add(1);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 24) as u8
    };
    let y: Vec<u8> = (0..w * h).map(|_| next()).collect();
    let u: Vec<u8> = (0..w * h / 4).map(|_| next()).collect();
    let v: Vec<u8> = (0..w * h / 4).map(|_| next()).collect();
    (y, u, v)
}

fn encode_one(encoder: &mut x264::Encoder, pts: i64, seed: u32) -> usize {
    let (y, u, v) = synthetic_frame(seed);
    let planes = [
        Plane { stride: WIDTH, data: &y },
        Plane { stride: WIDTH / 2, data: &u },
        Plane { stride: WIDTH / 2, data: &v },
    ];
    let image = Image::new(Colorspace::I420, WIDTH, HEIGHT, &planes);
    let (data, _picture) = encoder.encode(pts, image).expect("encode failed");
    data.entirety().len()
}

#[test]
fn vbv_build_succeeds() {
    let encoder = Setup::preset(Preset::Ultrafast, Tune::None, false, true)
        .crf(23.0)
        .vbv(8_000, 8_000)
        .fps(30, 1)
        .build(Colorspace::I420, WIDTH, HEIGHT);
    assert!(encoder.is_ok(), "building a CRF+VBV encoder must succeed");
}

#[test]
fn reconfig_vbv_keeps_encoding() {
    let mut encoder = Setup::preset(Preset::Ultrafast, Tune::None, false, true)
        .crf(23.0)
        .vbv(8_000, 8_000)
        .fps(30, 1)
        .build(Colorspace::I420, WIDTH, HEIGHT)
        .expect("build failed");

    for pts in 0..5 {
        let n = encode_one(&mut encoder, pts, pts as u32 + 1);
        assert!(n > 0, "frame {} produced no output", pts);
    }

    // Tighten the cap, then loosen it again; the encoder must keep
    // producing output after each reconfig.
    encoder.reconfig_vbv(500, 500).expect("tighten reconfig failed");
    for pts in 5..10 {
        let n = encode_one(&mut encoder, pts, pts as u32 + 1);
        assert!(n > 0, "frame {} after tighten produced no output", pts);
    }

    encoder.reconfig_vbv(8_000, 8_000).expect("loosen reconfig failed");
    for pts in 10..15 {
        let n = encode_one(&mut encoder, pts, pts as u32 + 1);
        assert!(n > 0, "frame {} after loosen produced no output", pts);
    }
}
