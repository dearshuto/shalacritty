pub trait ChunkFactory {
    type Chunk;
    fn create(&self, offset: u64, size: u64) -> Self::Chunk;
}

pub struct DirtyTracker {
    atom_size: u64,
    ranges: Vec<std::ops::Range<u64>>,
}

impl DirtyTracker {
    /// 更新が発生したバイト範囲を登録します
    fn add(&mut self, offset: u64, size: u64) {
        let start = offset;
        let end = offset + size;
        self.ranges.push(start..end);
    }

    fn into_chunks<F: ChunkFactory>(mut self, factory: F) -> Vec<F::Chunk> {
        if self.ranges.is_empty() {
            return vec![];
        }

        // 1. 開始地点でソート
        self.ranges.sort_by_key(|r| r.start);

        // 2. 近接または重複する範囲をマージ
        let mut merged = Vec::<std::ops::Range<u64>>::new();
        for r in self.ranges.drain(..) {
            if let Some(last) = merged.last_mut()
                // atom_size 以内の隙間なら結合してしまったほうが flush 回数を減らせる
                && r.start <= last.end + self.atom_size
            {
                last.end = last.end.max(r.end);
                continue;
            }

            merged.push(r);
        }

        merged
            .iter()
            .map(|range| {
                // 開始位置はアラインメントサイズに切り下げ
                let aligned_start = range.start - (range.start % self.atom_size);
                // 末端はアラインメントサイズに切り上げ
                let aligned_end = range.end.next_multiple_of(self.atom_size);
                let offset = aligned_start;
                let size = aligned_end - aligned_start;
                factory.create(offset, size)
            })
            .collect()
    }
}
