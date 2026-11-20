use blp::{AnyImage, Blp, EncodeMipOptions, EncodeOptions, Encoding, TextureType, Version};

/// A 64×32 image: a color gradient with an alpha ramp.
fn gradient() -> (Vec<u8>, u32, u32) {
    let (w, h) = (64u32, 32u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&[(x * 4) as u8, (y * 8) as u8, 200, (x * 4) as u8]);
        }
    }
    (rgba, w, h)
}

#[cfg(feature = "encode")]
#[test]
fn encode_then_decode_keeps_colors_and_alpha() {
    let (rgba, w, h) = gradient();
    let bytes = Blp::encode_from_rgba(&rgba, w, h, 100, &[]).unwrap();
    let (blp, mips) = Blp::parse(&bytes).unwrap();
    assert_eq!((blp.version, blp.texture_type, blp.alpha_bits), (Version::BLP1, TextureType::JPEG, 8));
    assert_eq!(blp.encoding().unwrap(), Encoding::Jpeg);
    assert_eq!((blp.width, blp.height), (w, h));
    // 64×32 halves down to 1×1 in 7 levels.
    assert_eq!(mips.len(), 7);
    assert_eq!((mips[6].width, mips[6].height), (1, 1));

    let base = blp.decode(&mips[0], &bytes).unwrap();
    let worst = base
        .as_raw()
        .iter()
        .zip(&rgba)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(worst <= 12, "JPEG at quality 100 drifted by {worst}");
    for mip in &mips {
        let img = blp.decode(mip, &bytes).unwrap();
        assert_eq!(img.dimensions(), (mip.width, mip.height));
    }
}

#[cfg(feature = "encode")]
#[test]
fn opaque_image_has_no_alpha() {
    let (mut rgba, w, h) = gradient();
    rgba.chunks_exact_mut(4)
        .for_each(|p| p[3] = 255);
    let bytes = Blp::encode_from_rgba(&rgba, w, h, 90, &[]).unwrap();
    let (blp, mips) = Blp::parse(&bytes).unwrap();
    assert_eq!(blp.alpha_bits, 0);
    assert!(
        blp.decode(&mips[0], &bytes)
            .unwrap()
            .pixels()
            .all(|p| p[3] == 255)
    );
}

#[cfg(feature = "encode")]
#[test]
fn skipped_mips_leave_holes_and_later_ones_are_kept() {
    let (rgba, w, h) = gradient();
    let opts = EncodeMipOptions { specific_mips: Some(vec![true, true, false, true]), ..Default::default() };
    let img = AnyImage::from_rgba(rgba, w, h).unwrap();
    let bytes = img
        .encode(&EncodeOptions::Blp { quality: 80, mip_options: Some(opts), raw: None })
        .unwrap();
    let (blp, mips) = Blp::parse(&bytes).unwrap();
    // Missing entries count as wanted: every level but mip 2.
    assert_eq!(mips.len(), 7);
    assert_eq!(mips[2].length, 0);
    assert!(blp.decode(&mips[2], &bytes).is_err());
    assert!(blp.decode(&mips[3], &bytes).is_ok());
}

#[cfg(feature = "encode")]
#[test]
fn min_size_stops_the_chain() {
    let (rgba, w, h) = gradient();
    let opts = EncodeMipOptions { min_size: Some(8), ..Default::default() };
    let img = AnyImage::from_rgba(rgba, w, h).unwrap();
    let bytes = img
        .encode(&EncodeOptions::Blp { quality: 80, mip_options: Some(opts), raw: None })
        .unwrap();
    let any = AnyImage::from_buffer(&bytes).unwrap();
    // 64×32, 32×16, 16×8: the next one would be 8×4.
    assert_eq!(any.frames.len(), 3);
    let mips = any.decode_frames().unwrap();
    assert_eq!(
        mips.iter()
            .map(|m| m.dimensions())
            .collect::<Vec<_>>(),
        [(64, 32), (32, 16), (16, 8)]
    );
}

#[cfg(feature = "encode")]
#[test]
fn raw_mip_is_a_standalone_jpeg() {
    let (rgba, w, h) = gradient();
    let bytes = Blp::encode_from_rgba(&rgba, w, h, 80, &[]).unwrap();
    let any = AnyImage::from_buffer(&bytes).unwrap();
    let jpeg = any
        .encode(&EncodeOptions::Blp { quality: 0, mip_options: None, raw: Some(1) })
        .unwrap();
    assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    let mut dec = jpeg_decoder::Decoder::new(jpeg.as_slice());
    dec.decode().unwrap();
    assert_eq!(dec.info().map(|i| (i.width, i.height)), Some((32, 16)));
}

#[cfg(feature = "encode")]
#[test]
fn non_power_of_two_is_scaled_up() {
    let rgba = vec![128u8; 3 * 5 * 4];
    let bytes = Blp::encode_from_rgba(&rgba, 3, 5, 80, &[]).unwrap();
    let (blp, _) = Blp::parse(&bytes).unwrap();
    assert_eq!((blp.width, blp.height), (4, 8));
}

/// A BLP1 palette texture written by hand: 2×2, 8-bit alpha, one mip.
fn palette_blp() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"BLP1");
    for v in [1u32, 8, 2, 2, 4, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let data_at = 156 + 1024;
    let mut offsets = [0u32; 16];
    let mut lengths = [0u32; 16];
    offsets[0] = data_at;
    lengths[0] = 8;
    offsets
        .iter()
        .chain(&lengths)
        .for_each(|v| b.extend_from_slice(&v.to_le_bytes()));
    let mut palette = vec![0u8; 1024];
    palette[4..8].copy_from_slice(&[0, 0, 255, 0]); // 1: red
    palette[8..12].copy_from_slice(&[255, 0, 0, 0]); // 2: blue
    b.extend_from_slice(&palette);
    b.extend_from_slice(&[1, 2, 2, 1, 255, 128, 64, 0]);
    b
}

#[test]
fn palette_texture_decodes() {
    let bytes = palette_blp();
    let (blp, mips) = Blp::parse(&bytes).unwrap();
    assert_eq!(blp.encoding().unwrap(), Encoding::Palette);
    // The header says 2×2: two levels, the second missing.
    assert_eq!(mips.len(), 1);
    assert_eq!(blp.holes, 0);
    let img = blp.decode(&mips[0], &bytes).unwrap();
    assert_eq!(img.as_raw(), &[255, 0, 0, 255, 0, 0, 255, 128, 0, 0, 255, 64, 255, 0, 0, 0]);
}

#[test]
fn truncated_input_is_an_error_not_a_panic() {
    let bytes = palette_blp();
    for len in 0..bytes.len() {
        let cut = &bytes[..len];
        if let Ok((blp, mips)) = Blp::parse(cut) {
            let _ = blp.decode(&mips[0], cut);
        }
    }
}

/// A BLP2 DXT1 texture: 4×4, one block of solid white.
#[test]
fn blp2_dxt1_decodes() {
    let mut b = Vec::new();
    b.extend_from_slice(b"BLP2");
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&[2, 0, 0, 0]);
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&4u32.to_le_bytes());
    let mut table = [0u32; 32];
    table[0] = 148 + 1024;
    table[16] = 8;
    table
        .iter()
        .for_each(|v| b.extend_from_slice(&v.to_le_bytes()));
    b.extend_from_slice(&[0u8; 1024]);
    b.extend_from_slice(&[0xFF, 0xFF, 0, 0, 0, 0, 0, 0]);

    let (blp, mips) = Blp::parse(&b).unwrap();
    assert_eq!(blp.encoding().unwrap(), Encoding::Dxt1);
    let img = blp.decode(&mips[0], &b).unwrap();
    assert!(
        img.pixels()
            .all(|p| p.0 == [255, 255, 255, 255])
    );
}
