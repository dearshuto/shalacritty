use crate::services::utils::diff_calculator::Diff;

pub trait Patch<T> {
    fn new(index: usize, value: T) -> Self;
}

pub fn generate_patch<T, P>(diff_iterator: impl Iterator<Item = Diff<T>>) -> Vec<P>
where
    T: Default,
    P: Patch<T>,
{
    let mut patches = Vec::<P>::default();
    let mut old_content_index = 0;
    let mut new_content_index = 0;

    for diff in diff_iterator {
        match diff {
            Diff::Common(content) => {
                if old_content_index != new_content_index {
                    let patch = P::new(new_content_index, content);
                    patches.push(patch);
                }
                old_content_index += 1;
                new_content_index += 1;
            }
            Diff::Add(content) => {
                let patch = P::new(new_content_index, content);
                patches.push(patch);
                new_content_index += 1;
            }
            Diff::Remove(_) => {
                old_content_index += 1;
            }
        }
    }

    patches
}

#[cfg(test)]
mod tests {
    use crate::services::utils::{self, Patch};

    struct PatchData {
        pub index: usize,
        pub char: char,
    }

    impl Patch<char> for PatchData {
        fn new(index: usize, value: char) -> Self {
            Self { index, char: value }
        }
    }

    fn test_impl(old: impl Iterator<Item = char>, new: impl Iterator<Item = char>) {
        let mut old: Vec<_> = old.collect();
        let new: Vec<_> = new.collect();
        if old.len() < new.len() {
            old.resize(new.len(), Default::default());
        } else if old.len() > new.len() {
            old.truncate(new.len());
        }
        let diff_collectin = utils::diff_calculator::calculate_diff(old.as_slice(), new.as_slice());

        let patches: Vec<PatchData> = utils::generate_patch(diff_collectin.into_iter());
        for patch in &patches {
            old[patch.index] = patch.char;
        }
        assert_eq!(old, new);
    }

    #[test]
    fn equal() {
        let old = ['a', 'b', 'c', 'd'];
        let new = ['a', 'b', 'c', 'd'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn add_tail() {
        let old = ['a', 'b'];
        let new = ['a', 'b', 'c', 'd', 'e'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn add_head() {
        let old = ['d', 'e'];
        let new = ['a', 'b', 'c', 'd', 'e'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn replace_middle() {
        let old = ['a', 'b', 'c', 'd', 'e'];
        let new = ['a', 'x', 'y', 'd', 'e'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn remove_head() {
        let old = ['a', 'b', 'c', 'd', 'e'];
        let new = ['c', 'd', 'e'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn remove_tail() {
        let old = ['a', 'b', 'c', 'd', 'e'];
        let new = ['a', 'b', 'c'];
        test_impl(old.into_iter(), new.into_iter());
    }

    #[test]
    fn remove_middle() {
        let old = ['a', 'b', 'c', 'd', 'e'];
        let new = ['a', 'e'];
        test_impl(old.into_iter(), new.into_iter());
    }
}
