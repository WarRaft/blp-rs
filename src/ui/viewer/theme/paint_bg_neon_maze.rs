use eframe::egui::{epaint::Shape, *};

pub fn paint_bg_neon_maze(ctx: &Context, seed: u64) {
    let painter = ctx.layer_painter(LayerId::background());
    let rect = ctx.content_rect();

    // геометрия сетки
    let target = 22.0_f32.max(8.0);
    let cols = (rect.width() / target).floor().max(1.0) as i32;
    let rows = (rect.height() / target)
        .floor()
        .max(1.0) as i32;

    let size = (rect.width() / cols as f32).min(rect.height() / rows as f32);
    // сдвигаем, учитывая буфер по 1 клетке
    let offset = pos2(rect.left() + (rect.width() - cols as f32 * size) * 0.5 - size, rect.top() + (rect.height() - rows as f32 * size) * 0.5 - size);

    let line_width: f32 = 10.0;
    let stroke = Stroke { width: line_width, color: Color32::from_rgba_unmultiplied(0, 22, 25, 255) };
    let extend = line_width * 0.5;

    // ===== 1) генерим диагонали с запретом ромбов =====
    // true  => "\"  (TL -> BR)
    // false => "/"  (BL -> TR)
    let bw = (cols + 2) as usize; // +2 по X
    let bh = (rows + 2) as usize; // +2 по Y
    let mut grid = vec![vec![false; bw]; bh];

    for y in 0..bh {
        for x in 0..bw {
            // базовый рандом
            let mut v = (mix64(seed ^ ((x as u64) << 32) ^ (y as u64)) & 1) == 0;

            if x > 0 && y > 0 {
                let nw = grid[y - 1][x - 1];
                let n = grid[y - 1][x];
                let w = grid[y][x - 1];

                // если сейчас закроем ромб (/\ затем \/ или наоборот) — переворачиваем
                if (nw != n) && (nw != w) && (v == nw) {
                    v = !v;
                }
            }
            grid[y][x] = v;
        }
    }

    // ===== 2) рисуем =====
    for y in 0..bh as i32 {
        for x in 0..bw as i32 {
            let x0 = offset.x + x as f32 * size;
            let y0 = offset.y + y as f32 * size;
            let x1 = x0 + size;
            let y1 = y0 + size;

            let v = grid[y as usize][x as usize];
            // v=true => "\"; v=false => "/"
            let (a, b) = if v { (pos2(x0, y0), pos2(x1, y1)) } else { (pos2(x0, y1), pos2(x1, y0)) };

            let mut dir = b - a;
            let len = dir.length().max(1e-6);
            dir /= len;

            let a_ext = a - dir * extend;
            let b_ext = b + dir * extend;

            painter.add(Shape::line_segment([a_ext, b_ext], stroke));
        }
    }
}

// как у тебя
#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
