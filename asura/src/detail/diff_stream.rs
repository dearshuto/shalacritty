use crate::terminal_emulator::{DiffContent, DiffType};

use itertools::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ContentCache {
    pub code: char,
    pub x: usize,
    pub y: i32,
    pub fg: [u8; 3],
}

/// 差分検出器
pub struct DiffStream {
    cache: Vec<ContentCache>,
}

impl DiffStream {
    pub fn new() -> Self {
        Self {
            cache: Vec::default(),
        }
    }

    pub fn calculate<U>(&mut self, sequence: impl Iterator<Item = U>) -> Vec<DiffType>
    where
        U: Into<(ContentCache, DiffContent)> + PartialEq<ContentCache>,
    {
        // テストしやすいように実装の詳細は副作用のない静的な関数として定義
        calculate(&mut self.cache, sequence)
    }
}

fn calculate<U>(
    cache_container: &mut Vec<ContentCache>,
    sequence: impl Iterator<Item = U>,
) -> Vec<DiffType>
where
    U: Into<(ContentCache, DiffContent)> + PartialEq<ContentCache>,
{
    let mut diff_contents = Vec::default();
    let cache_len = cache_container.len();

    // 新たなシーケンスとキャッシュで新旧のペアを作って走査する
    // どちらかが尽きたインデックスを返して、残りを全追加か全削除かで処理する
    // 旧の方はミュータブル制約のため走査できないのでインデックスで取りまわす
    let old_iter = 0..cache_len;
    let mut iter = sequence.zip_longest(old_iter).enumerate();

    let shortest_index = loop {
        let Some((index, new_old)) = iter.next() else {
            // 両方同時に枯渇したので新旧が同じ長さだった
            break None;
        };

        let (new, old) = new_old.left_and_right();

        let Some(new) = new else {
            // 新たな値が先に枯渇した
            break Some(index);
        };

        let Some(old) = old else {
            // 古い値が先に枯渇したので残りはすべて「追加」で処理する
            // イテレーターが進んでいるので今回の走査分は処理しておく
            let (cache, output) = new.into();
            cache_container.push(cache);
            diff_contents.push(DiffType::Add(output));

            break Some(index);
        };

        // 差分がなかった
        if new.eq(&cache_container[old]) {
            continue;
        }

        // 差分が発生した
        // キャッシュ更新
        let (cache, output) = new.into();
        cache_container[index] = cache;

        // 返り値更新
        diff_contents.push(DiffType::Update(output));

        break Some(index);
    };

    // 新旧のシーケンスの長さが同じだったので差分検出だけで済んだ
    let Some(shortest_index) = shortest_index else {
        return diff_contents;
    };

    if shortest_index < cache_len {
        // 新たな値が短くなってた
        // 短くなった分を削除として処理する
        let mut removed = (shortest_index..cache_len)
            .map(|index| DiffType::Remove(index))
            .collect();
        diff_contents.append(&mut removed);

        // キャッシュの更新
        // 使用済みサイズを切り詰めれば更新完了
        cache_container.truncate(shortest_index);
    } else {
        // 新たな値が長くなっていたので残りすべてを「追加」で処理する
        for (_index, new_old) in iter {
            let new = new_old.left().unwrap();
            let (cache, output) = new.into();
            cache_container.push(cache);
            diff_contents.push(DiffType::Add(output));
        }
    }

    diff_contents
}

#[cfg(test)]
mod tests {
    use crate::Content;

    use super::*;

    struct EnumerableContentCache((usize, ContentCache));

    impl Into<(ContentCache, DiffContent)> for EnumerableContentCache {
        fn into(self) -> (ContentCache, DiffContent) {
            let content_cache = self.0 .1;

            let diff_content = DiffContent {
                index: self.0 .0,
                content: Content {
                    code: content_cache.code,
                    x: content_cache.x,
                    y: content_cache.y,
                    fg: crate::detail::into_snorm(&content_cache.fg),
                },
            };
            (content_cache, diff_content)
        }
    }

    impl PartialEq<ContentCache> for EnumerableContentCache {
        fn eq(&self, other: &ContentCache) -> bool {
            self.0 .1.eq(other)
        }
    }

    #[test]
    fn empty() {
        let diff_types =
            super::calculate::<EnumerableContentCache>(&mut Vec::default(), [].into_iter());
        assert!(diff_types.is_empty());
    }

    #[test]
    fn add_only() {
        let new = [ContentCache {
            code: 'a',
            x: 1,
            y: 2,
            fg: [255, 255, 255],
        }]
        .into_iter()
        .enumerate()
        .map(|x| EnumerableContentCache(x));

        let diff_types = super::calculate(&mut Vec::default(), new);
        assert_eq!(
            diff_types,
            [DiffType::Add(DiffContent {
                index: 0,
                content: Content {
                    code: 'a',
                    x: 1,
                    y: 2,
                    fg: [1.0, 1.0, 1.0]
                }
            })]
        );
    }

    #[test]
    fn add_multiple() {
        let new = [
            ContentCache {
                code: 'a',
                x: 1,
                y: 2,
                fg: [255, 255, 255],
            },
            ContentCache {
                code: 'b',
                x: 3,
                y: 4,
                fg: [255, 255, 255],
            },
        ]
        .into_iter()
        .enumerate()
        .map(|x| EnumerableContentCache(x));

        let diff_types = super::calculate(&mut Vec::default(), new);
        assert_eq!(
            diff_types,
            [
                DiffType::Add(DiffContent {
                    index: 0,
                    content: Content {
                        code: 'a',
                        x: 1,
                        y: 2,
                        fg: [1.0, 1.0, 1.0]
                    }
                }),
                DiffType::Add(DiffContent {
                    index: 1,
                    content: Content {
                        code: 'b',
                        x: 3,
                        y: 4,
                        fg: [1.0, 1.0, 1.0]
                    }
                })
            ]
        );
    }
}
