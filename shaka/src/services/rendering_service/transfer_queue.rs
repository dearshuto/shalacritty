pub struct TransferQueue<T: Copy> {
    buffer: Vec<T>,
}

impl<T> TransferQueue<T>
where
    T: Copy,
{
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push(&mut self, dst_buffer: &mut [T], src_buffer: &[T]) -> usize {
        self.buffer.extend_from_slice(src_buffer);
        self.pop(dst_buffer)
    }

    pub fn pop(&mut self, dst_buffer: &mut [T]) -> usize {
        let copy_count = dst_buffer.len().min(self.buffer.len());
        dst_buffer[0..copy_count].copy_from_slice(&self.buffer[0..copy_count]);
        self.buffer.drain(0..copy_count);
        copy_count
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::services::rendering_service::transfer_queue::TransferQueue;

    #[test]
    fn test_simple() {
        let mut queue = TransferQueue::new();
        let mut dst = [0u8; 10];

        // 出力バッファー数 2 に対して 5 この要素を追加したので追加できたのは 2 個
        // 残りの 3 つは内部のバッファーに保持される
        assert_eq!(queue.push(&mut dst, &[0, 1, 2, 3, 4]), 5);
        assert_eq!(dst[0..5], [0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_transfer_queue() {
        let mut queue = TransferQueue::new();
        let mut dst = [0u8; 2];

        // 出力バッファー数 2 に対して 5 この要素を追加したので追加できたのは 2 個
        // 残りの 3 つは内部のバッファーに保持される
        assert_eq!(queue.push(&mut dst, &[0, 1, 2, 3, 4]), 2);
        assert_eq!(dst, [0, 1]);

        // pop すると内部データから値が返ってくる
        assert_eq!(queue.pop(&mut dst), 2);
        assert_eq!(dst, [2, 3]);

        // pop すると内部データから値が返ってくる
        assert_eq!(queue.pop(&mut dst), 1);
        assert_eq!(dst[0..1], [4]);
    }
}
