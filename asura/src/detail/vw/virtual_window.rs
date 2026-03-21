use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId {
    internal: uuid::Uuid,
}

impl TileId {
    fn new() -> Self {
        Self {
            internal: uuid::Uuid::now_v7(),
        }
    }
}

enum Orientation {
    Horizontal,
    #[allow(unused)]
    Vertical,
}

struct Tile {
    #[allow(unused)]
    width: f32,
    #[allow(unused)]
    height: f32,
}

#[derive(Clone, Copy)]
pub struct ActualSize {
    pub width: u32,
    pub height: u32,
}

pub struct VirtualWindow {
    // 番兵用の分割領域
    root_id: TileId,

    // 仮想ウィンドウ内の分割領域
    tile_table: HashMap<TileId, Tile>,

    // 分割領域の親子関係
    hierarchy_table: HashMap<TileId, Vec<TileId>>,

    orientation_table: HashMap<TileId, Orientation>,

    // 領域が実際に占有するサイズ
    actual_size_table: HashMap<TileId, ActualSize>,
}

impl VirtualWindow {
    #[allow(unused)]
    pub fn new(width: u32, height: u32) -> (TileId, Self) {
        // 番兵
        let root_id = TileId::new();
        let root_tile = Tile {
            width: 1.0,
            height: 1.0,
        };

        // ルート直下の初期タイル
        let id = TileId::new();
        let tile = Tile {
            width: 1.0,
            height: 1.0,
        };

        // 1. ルート要素に番兵
        // 2. ルート直下に初期タイル
        let hierarchy_table = HashMap::from([(root_id, vec![id]), (id, Vec::default())]);

        // 初期状態で実際のサイズは計算するまでもなく初期値なので、
        // 決め打ちした値で HashMap を用意
        let actual_size_table = HashMap::from([
            (root_id, ActualSize { width, height }),
            (id, ActualSize { width, height }),
        ]);

        (
            id,
            Self {
                root_id,
                tile_table: HashMap::from([(root_id, root_tile), (id, tile)]),
                hierarchy_table,
                orientation_table: HashMap::from([(root_id, Orientation::Horizontal)]),
                actual_size_table,
            },
        )
    }

    #[allow(unused)]
    pub fn get_actual_size(&self, id: TileId) -> Option<ActualSize> {
        let Some(actual_size) = self.actual_size_table.get(&id) else {
            return None;
        };

        Some(*actual_size)
    }

    #[allow(unused)]
    pub fn resize(&mut self, width: u32, height: u32) {
        // ルートに反映
        let root_id = self.root_id;
        let root_tile_size = self.actual_size_table.get_mut(&root_id).unwrap();
        root_tile_size.width = width;
        root_tile_size.height = height;

        // 子供達に反映
        self.calculate_actual_size();
    }

    #[allow(unused)]
    pub fn remove(&mut self, id: TileId) -> bool {
        // 必ず 1 つは分割領域が存在するようにしたいので、
        // 番兵用のルート要素とその直下のタイルしかない場合は消さない
        if self.tile_table.len() == 2 {
            return false;
        }

        // ないと思うが存在しないタイルを削除しようとした場合は失敗扱い
        let Some(_tile) = self.tile_table.remove(&id) else {
            return false;
        };

        // 階層を消去
        let mut new_children = self.hierarchy_table.remove(&id).unwrap();
        for (_key, children) in &mut self.hierarchy_table {
            // 削除対象を子供にもつ要素を探す
            let Some(position) = children.iter().position(|x| *x == id) else {
                continue;
            };

            // 削除対象を削除しつつ、削除対象の子供の親を付け替える
            let _ = children.swap_remove(position);
            children.append(&mut new_children);

            // ソートして...
            children.sort();

            // 再計算
            self.calculate_actual_size();

            return true;
        }

        false
    }

    #[allow(unused)]
    pub fn split_horizontal(&mut self, id: TileId) -> Option<TileId> {
        let mut parent_id = None;
        for (p_id, children) in &self.hierarchy_table {
            if children.contains(&id) {
                parent_id = Some(*p_id);
                break;
            }
        }

        let Some(parent_id) = parent_id else {
            return None;
        };

        let Some(children) = self.hierarchy_table.get_mut(&parent_id) else {
            return None;
        };

        // 元のタイルのインデックスを見つける
        let Some(index) = children.iter().position(|&child_id| child_id == id) else {
            return None;
        };

        let new_tile_id = TileId::new();
        let new_tile = Tile {
            width: 1.0,
            height: 1.0,
        };

        self.tile_table.insert(new_tile_id, new_tile);

        // 新しいタイルの初期サイズを設定
        let parent_actual_size = *self.actual_size_table.get(&parent_id).unwrap();
        self.actual_size_table.insert(new_tile_id, parent_actual_size);

        // 新しいタイルを元のタイルの隣に挿入
        children.insert(index + 1, new_tile_id);

        // 親のオリエンテーションをHorizontalに設定
        self.orientation_table
            .insert(parent_id, Orientation::Horizontal);

        // 新しいタイルの階層エントリを作成
        self.hierarchy_table
            .insert(new_tile_id, Vec::default());

        self.calculate_actual_size();

        Some(new_tile_id)
    }

    /// ルートから葉に向かってサイズを計算していきます
    fn calculate_actual_size(&mut self) {
        let mut ids = vec![self.root_id];

        while let Some(id) = ids.pop() {
            let actual_size = if let Some(t) = self.actual_size_table.get(&id) {
                Some(*t)
            } else {
                None
            };

            let Some(actual_size) = actual_size else {
                continue;
            };

            let Some(children) = self.hierarchy_table.get_mut(&id) else {
                continue;
            };

            let Some(orientation) = self.orientation_table.get(&id) else {
                continue;
            };

            let len = children.len() as u32;
            for child in children {
                let size = self.actual_size_table.get_mut(child).unwrap();
                match orientation {
                    Orientation::Horizontal => {
                        size.width = actual_size.width;
                        size.height = actual_size.height / len
                    }
                    Orientation::Vertical => {
                        size.height = actual_size.height;
                        size.width = actual_size.width / len
                    }
                };

                ids.push(*child);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::VirtualWindow;

    #[test]
    fn new() {
        let (id, vw) = VirtualWindow::new(640, 480);

        let actual_size = vw.get_actual_size(id).unwrap();
        assert_eq!(actual_size.width, 640);
        assert_eq!(actual_size.height, 480);
    }

    #[test]
    fn remove_last() {
        let (id, mut vw) = VirtualWindow::new(640, 480);

        // 最後の要素は消せない
        assert!(!vw.remove(id));
    }

    #[test]
    fn resize() {
        let (id, mut vw) = VirtualWindow::new(640, 480);

        vw.resize(1280, 960);

        let actual_size = vw.get_actual_size(id).unwrap();
        assert_eq!(actual_size.width, 1280);
        assert_eq!(actual_size.height, 960);
    }

    #[test]
    fn split_horizontal() {
        let (id, mut vw) = VirtualWindow::new(640, 480);
        let new_id = vw.split_horizontal(id).unwrap();

        // ちゃんと新しい ID が生成されているか
        assert_ne!(id, new_id);

        // サイズが半分になっているか
        let actual_size = vw.get_actual_size(id).unwrap();
        assert_eq!(actual_size.width, 640);
        assert_eq!(actual_size.height, 240);

        let new_actual_size = vw.get_actual_size(new_id).unwrap();
        assert_eq!(new_actual_size.width, 640);
        assert_eq!(new_actual_size.height, 240);
    }
    #[test]
    fn split_horizontal_and_remove() {
        let (id, mut vw) = VirtualWindow::new(640, 480);

        let new_tile_id = vw.split_horizontal(id).unwrap();
        vw.remove(new_tile_id);

        // 分割した後に消したらまた全画面分に戻る
        let actual_size = vw.get_actual_size(id).unwrap();
        assert_eq!(actual_size.width, 640);
        assert_eq!(actual_size.height, 480);

        // 元から存在したタイルを削除した場合は残ったタイルが全画面になる
        let new_tile_id = vw.split_horizontal(id).unwrap();
        vw.remove(id);
        let actual_size = vw.get_actual_size(new_tile_id).unwrap();
        assert_eq!(actual_size.width, 640);
        assert_eq!(actual_size.height, 480);
    }
}
