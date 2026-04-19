pub fn compute_grid_size(window_size: [u32; 2], font_size: f32) -> (u32, u32) {
    let font_width = font_size / 2.0;
    let cols = (window_size[0] as f32 / font_width) as u32;
    let rows = (window_size[1] as f32 / font_size) as u32;
    (cols, rows)
}

pub fn compute_world_matrix(
    window_size: [u32; 2],
    font_size: f32,
    grid_x: u32,
    grid_y: u32,
) -> nalgebra::Matrix3<f32> {
    let (cols, rows) = compute_grid_size(window_size, font_size);
    let font_width = font_size / 2.0;

    let width = window_size[0] as f32;
    let height = window_size[1] as f32;

    // グリッド全体のサイズを計算して余白（センタリング用）を求める
    let total_grid_width = cols as f32 * font_width;
    let total_grid_height = rows as f32 * font_size;

    let offset_x = (width - total_grid_width) / 2.0;
    let offset_y = (height - total_grid_height) / 2.0;

    // ローカル座標の単位正方形（幅1, 高さ1, 中心(0,0)）を
    // NDC空間（[-1, 1]）上の対応するグリッド位置へ変換する。

    // スケーリング：単位サイズをセルサイズに変換し、NDCの全幅2で正規化
    let sx = (font_width / width) * 2.0;
    let sy = (font_size / height) * 2.0;

    // 平行移動：ピクセル単位のセル中心座標を求め、NDCの [-1, 1] に変換
    let px = offset_x + (grid_x as f32 + 0.5) * font_width;
    let py = offset_y + (grid_y as f32 + 0.5) * font_size;

    let tx = (px / width) * 2.0 - 1.0;
    let ty = (py / height) * 2.0 - 1.0;

    nalgebra::Matrix3::new(
        sx, 0.0, tx, //
        0.0, sy, ty, //
        0.0, 0.0, 1.0, //
    )
}
