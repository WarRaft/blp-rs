use crate::core::image::{ImageBlp, MAX_MIPS};
use crate::core::mipmap::Mipmap;
use crate::core::types::SourceKind;
use crate::error::error::BlpError;
use image;
use image::GenericImageView;

const MAX_POW2: u32 = 8192; // при необходимости скорректируй верхнюю границу

fn pow2_list_up_to(max_v: u32) -> Vec<u32> {
    let mut v = 1u32;
    let mut out = Vec::new();
    while v <= max_v {
        out.push(v);
        if v == u32::MAX / 2 {
            break;
        }
        v <<= 1;
    }
    out
}

/// Выбираем целевой кадр (W*, H*) — степени двойки.
/// Критерии (лексикографически):
///   1) минимальный масштаб s = max(W*/w0, H*/h0)  (не искажаем, «минимально дотянуть»)
///   2) минимальная разница в соотношении сторон |(W*/H*) - (w0/h0)|
///   3) минимальная площадь W* * H*
/// Возвращает (W*, H*).
fn pick_pow2_cover(w0: u32, h0: u32) -> (u32, u32) {
    debug_assert!(w0 > 0 && h0 > 0);
    let ws = pow2_list_up_to(MAX_POW2);
    let hs = pow2_list_up_to(MAX_POW2);

    let w0f = w0 as f64;
    let h0f = h0 as f64;
    let ar0 = w0f / h0f;

    let mut best = None::<(f64, f64, u64, u32, u32)>; // (s, ar_diff, area, W, H)

    for &ww in &ws {
        // если совсем маленькие степени двойки — пропустим очевидно меньше исходника:
        // НО мы разрешаем и «подкадр» меньше исходника (это повысит s), так что не фильтруем.
        for &hh in &hs {
            let s = (ww as f64 / w0f).max(hh as f64 / h0f); // масштаб cover
            if s < 1.0 {
                // Нужно обязательно покрыть кадр — если s<1, картинка не закроет кадр.
                continue;
            }
            let ar = ww as f64 / hh as f64;
            let ar_diff = (ar - ar0).abs();
            let area = (ww as u64) * (hh as u64);

            let cand = (s, ar_diff, area, ww, hh);
            match best {
                None => best = Some(cand),
                Some(cur) => {
                    // сравнение: s, затем ar_diff, затем area
                    if cand.0 < cur.0 || (cand.0 == cur.0 && (cand.1 < cur.1 || (cand.1 == cur.1 && cand.2 < cur.2))) {
                        best = Some(cand);
                    }
                }
            }
        }
    }

    if let Some((_s, _ard, _area, ww, hh)) = best { (ww, hh) } else { (w0, h0) }
}

impl ImageBlp {
    /// Лёгкий путь для «случайного изображения»: только разметка без RGBA.
    /// 1) Считываем исходные размеры
    /// 2) Выбираем целевой кадр (W*,H*) — степени двойки по правилу «минимум апскейла» и «минимум кропа»
    /// 3) Формируем цепочку мипов (только width/height), image=None
    ///    Хвост после 1×1 заполняем 0×0 (а не 1×1).
    pub fn from_buf_image(buf: &[u8]) -> Result<Self, BlpError> {
        let dyn_img = image::load_from_memory(buf)?;
        let (w0, h0) = dyn_img.dimensions();
        if w0 == 0 || h0 == 0 {
            return Err(BlpError::new("error-image-empty")
                .with_arg("width", w0)
                .with_arg("height", h0));
        }

        let (base_w, base_h) = pick_pow2_cover(w0, h0);

        // Сколько уровней до 1×1 включительно:
        // floor(log2(max)) + 1  ==  32 - leading_zeros(max)  (для u32)
        let levels = (32 - base_w.max(base_h).leading_zeros()) as usize;

        let mut mipmaps = Vec::with_capacity(MAX_MIPS);
        let (mut w, mut h) = (base_w, base_h);

        for i in 0..MAX_MIPS {
            if i < levels {
                mipmaps.push(Mipmap {
                    width: w,
                    height: h,
                    image: None, // НЕ создаём RgbaImage
                    offset: 0,
                    length: 0,
                });
                // делим пополам, но не ниже 1
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            } else {
                // хвост — отсутствующие уровни
                mipmaps.push(Mipmap::default());
            }
        }

        Ok(ImageBlp {
            width: base_w, //
            height: base_h,
            mipmaps,
            source: SourceKind::Image,
            ..Default::default()
        })
    }
}
