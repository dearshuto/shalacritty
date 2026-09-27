/// 切り出された 2D 矩形領域（ピクセル単位）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocateInfo {
    pub x: u32,
    pub y: u32,
}

/// 棚の管理構造体
#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u32,
    current_x: u32,
}

/// ピュアな 2D テクスチャアトラス・アロケーター
#[derive(Debug)]
pub struct AtlasAllocator {
    width: u32,
    height: u32,
    padding: u32,
    shelves: Vec<Shelf>,
    current_y: u32,
}

impl AtlasAllocator {
    /// アトラスの全体サイズとグリフ間のパディングを指定して初期化
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            width,
            height,
            padding,
            shelves: Vec::new(),
            current_y: 0,
        }
    }

    /// 要求されたサイズの領域を切り出す (Allocate)
    /// 空き容量が足りない場合は `None` を返す
    pub fn allocate(&mut self, req_width: u32, req_height: u32) -> Option<AllocateInfo> {
        // パディングを含めた必要な幅と高さ
        let padded_w = req_width + self.padding;
        let padded_h = req_height + self.padding;

        if padded_w > self.width || padded_h > self.height {
            return None; // そもそもアトラスのサイズを超えている
        }

        // 1. 既存の「棚（Shelf）」に収まるか探す（高さが合い、横幅に空きがあるか）
        for shelf in &mut self.shelves {
            // 高さのフィット感（無駄に高い棚に入れない）と横幅チェック
            if shelf.height >= padded_h && (self.width - shelf.current_x) >= padded_w {
                let alloc = AllocateInfo {
                    x: shelf.current_x,
                    y: shelf.y,
                };
                shelf.current_x += padded_w;
                return Some(alloc);
            }
        }

        // 2. 既存の棚に入らない場合、新しい棚（Shelf）を下に作る
        if self.height - self.current_y >= padded_h {
            let new_shelf = Shelf {
                y: self.current_y,
                height: padded_h,
                current_x: padded_w,
            };

            let alloc = AllocateInfo {
                x: 0,
                y: self.current_y,
            };

            self.current_y += padded_h;
            self.shelves.push(new_shelf);

            return Some(alloc);
        }

        // 3. アトラスが満杯（Out of Memory）
        None
    }

    /// アトラスを完全にリセット（全解放）する
    pub fn clear(&mut self) {
        self.shelves.clear();
        self.current_y = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_basic() {
        let mut allocator = AtlasAllocator::new(1024, 1024, 1);

        // 1つ目の文字 'A' (10x20)
        let a1 = allocator.allocate(10, 20).unwrap();
        assert_eq!(a1, AllocateInfo { x: 0, y: 0 });

        // 2つ目の文字 'B' (15x20) -> 同じ棚に並ぶはず
        let a2 = allocator.allocate(15, 20).unwrap();
        // パディング 1px が入るので x は 0 + 10 + 1 = 11
        assert_eq!(a2, AllocateInfo { x: 11, y: 0 });
    }

    #[test]
    fn test_out_of_memory() {
        let mut allocator = AtlasAllocator::new(32, 32, 0);
        assert!(allocator.allocate(32, 32).is_some());
        // もう空きがないので None が返る
        assert!(allocator.allocate(10, 10).is_none());
    }
}
