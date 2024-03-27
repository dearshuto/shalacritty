pub struct Diff<T> {
    items: Vec<T>,
    indeicies: Vec<usize>,
}

impl<T> Diff<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn indicies(&self) -> &[usize] {
        &self.indeicies
    }
}

pub trait IDiffCalculator<T> {
    fn calculate<TIterator>(&mut self, items: TIterator) -> Diff<T>
    where
        TIterator: Iterator<Item = T>;
}

pub struct DiffCalculator<T>
where
    T: Eq + Copy,
{
    old_items: Vec<T>,
}

impl<T> DiffCalculator<T>
where
    T: Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            old_items: Vec::default(),
        }
    }
}

impl<T> IDiffCalculator<T> for DiffCalculator<T>
where
    T: Eq + Copy,
{
    fn calculate<TIterator>(&mut self, items: TIterator) -> Diff<T>
    where
        TIterator: Iterator<Item = T>,
    {
        // 要素を列挙して比較するシンプルな実装
        // Wu の差分検出みたいないけてる実装に載せ替えたい

        let old_items: Vec<T> = items.collect();

        let mut changed_items = Vec::default();
        let mut item_indicies = Vec::default();
        for (index, item) in old_items.iter().enumerate() {
            let Some(old_item) = self.old_items.get(index) else {
                changed_items.push(*item);
                item_indicies.push(index);
                continue;
            };

            if old_item == item {
                continue;
            }

            changed_items.push(*item);
            item_indicies.push(index);
        }

        self.old_items = old_items;

        Diff {
            items: changed_items,
            indeicies: item_indicies,
        }
    }
}
