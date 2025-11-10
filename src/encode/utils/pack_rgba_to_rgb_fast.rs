#[inline(always)]
pub fn pack_rgba_to_rgb_fast(src: &[u8], w: usize, h: usize) -> (Vec<u8>, usize) {
    debug_assert_eq!(src.len(), w * h * 4);

    let mut out = vec![0u8; w * h * 3];
    let mut si = 0usize; // step 4
    let mut di = 0usize; // step 3

    // RGBA -> RGB (R,G,БЕЗ альфы)
    while si < src.len() {
        out[di] = src[si]; // R
        out[di + 1] = src[si + 1]; // G
        out[di + 2] = src[si + 2]; // B
        si += 4;
        di += 3;
    }
    (out, w * 3)
}
