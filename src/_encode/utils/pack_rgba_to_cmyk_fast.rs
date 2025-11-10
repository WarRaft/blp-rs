#[inline(always)]
pub fn pack_rgba_to_cmyk_fast(src: &[u8], w: usize, h: usize) -> (Vec<u8>, usize) {
    debug_assert_eq!(src.len(), w * h * 4);

    let mut out = vec![0u8; w * h * 4];
    let mut si = 0usize; // step 4
    let mut di = 0usize; // step 4

    // RGBA -> CMYK  (C=B, M=G, Y=R, K=A)
    while si < src.len() {
        out[di] = src[si + 2]; // C ← B
        out[di + 1] = src[si + 1]; // M ← G
        out[di + 2] = src[si]; // Y ← R
        out[di + 3] = src[si + 3]; // K ← A
        si += 4;
        di += 4;
    }
    (out, w * 4)
}
