#[derive(Debug)]
pub struct RangeAllocator {
    // 分割して割り当てる範囲
    width: u32,
    height: u32,

    // 未割り当て部分の開始点
    offset: [u32; 2],
    // 未割り当て部分の高さ
    head_height: u32,
}

impl RangeAllocator {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            offset: [0, 0],
            head_height: 0,
        }
    }

    pub fn allocate(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        // 横方向の限界を超えてたらそもそも割り当て不可能
        if self.width < width {
            return None;
        }

        // 横方向に入るか
        if (self.offset[0] + width) < self.width {
            // 横方向にヘッドを移動する
            let allocated_x = self.offset[0];
            self.offset[0] += width;

            // 横に追加したので　y 方向のヘッドは変わらない
            let allocated_y = self.offset[1];

            // 現在の行により高い領域を割り当てて場合は、
            // 次の行と被らないように禁止区域を伸ばしておく
            if self.head_height < (self.offset[1] + height) {
                self.head_height = self.offset[1] + height;
            }

            Some([allocated_x, allocated_y])
        } else {
            // 入らなかったら次の行に入れる
            // 横方向のヘッドは先頭に戻る
            let allocated_x = 0;
            self.offset[0] = width;

            let allocated_y = self.head_height;
            self.offset[1] = self.head_height;
            self.head_height += height;
            Some([allocated_x, allocated_y])
        }
    }

    pub fn invalidate(&mut self) {
        self.offset = [0, 0];
        self.head_height = 0;
    }
}
