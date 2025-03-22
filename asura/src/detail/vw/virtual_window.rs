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
    Vertical,
}

struct Tile {
    width: f32,
    height: f32,
}

struct ActualSize {
    width: u32,
    height: u32,
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

    pub fn resize(&mut self, width: u32, height: u32) {
        // ルートに反映
        let root_id = self.root_id;
        let root_tile_size = self.actual_size_table.get_mut(&root_id).unwrap();
        root_tile_size.width = width;
        root_tile_size.height = height;

        // 子供達に反映
        self.calculate_actual_size();
    }

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

            // ソートして終了
            children.sort();

            return true;
        }

        false
    }

    /// ルートから葉に向かってサイズを計算していきます
    fn calculate_actual_size(&mut self) {
        let mut ids = vec![self.root_id];

        while let Some(id) = ids.pop() {
            let Some(actual_size) = self.actual_size_table.get(&id) else {
                continue;
            };

            let Some(children) = self.hierarchy_table.get(&id) else {
                continue;
            };
        }
    }
}
