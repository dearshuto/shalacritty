#[derive(Debug, PartialEq, Clone)]
pub enum Diff<T> {
    Common(T),
    Add(T),
    Remove(T),
}

/// Myers' diff algorithm implementation.
pub fn calculate_diff<T: PartialEq + Clone>(old: &[T], new: &[T]) -> Vec<Diff<T>> {
    let n = old.len();
    let m = new.len();

    if n == 0 && m == 0 {
        return Vec::new();
    }

    if n == 0 {
        return new.iter().map(|v| Diff::Add(v.clone())).collect();
    }

    if m == 0 {
        return old.iter().map(|v| Diff::Remove(v.clone())).collect();
    }

    let max_d = n + m;
    let offset = max_d as isize;
    let mut v = vec![0isize; 2 * max_d + 1];
    let mut history = Vec::with_capacity(max_d + 1);

    for d in 0..=max_d {
        let d_isize = d as isize;
        for k in ((-d_isize)..=d_isize).step_by(2) {
            let k_idx = (k + offset) as usize;

            let mut x = if d == 0 {
                0
            } else if k == -d_isize || (k != d_isize && v[k_idx - 1] < v[k_idx + 1]) {
                v[k_idx + 1]
            } else {
                v[k_idx - 1] + 1
            };

            let mut y = x - k;

            while x < n as isize && y < m as isize && old[x as usize] == new[y as usize] {
                x += 1;
                y += 1;
            }

            v[k_idx] = x;

            if x >= n as isize && y >= m as isize {
                history.push(v.clone());
                return reconstruct_path(old, new, &history, d, k, offset);
            }
        }
        history.push(v.clone());
    }

    Vec::new()
}

fn reconstruct_path<T: PartialEq + Clone>(
    old: &[T],
    new: &[T],
    history: &[Vec<isize>],
    d_final: usize,
    mut k: isize,
    offset: isize,
) -> Vec<Diff<T>> {
    let mut diffs = Vec::new();
    let mut x = old.len() as isize;
    let mut y = new.len() as isize;

    for d in (0..=d_final).rev() {
        let d_isize = d as isize;

        let (prev_k, move_x) = if d == 0 {
            (0, 0)
        } else {
            let prev_v = &history[d - 1];
            if k == -d_isize
                || (k != d_isize
                    && prev_v[(k - 1 + offset) as usize] < prev_v[(k + 1 + offset) as usize])
            {
                (k + 1, prev_v[(k + 1 + offset) as usize])
            } else {
                (k - 1, prev_v[(k - 1 + offset) as usize] + 1)
            }
        };

        let move_y = move_x - k;

        while x > move_x && y > move_y {
            x -= 1;
            y -= 1;
            diffs.push(Diff::Common(old[x as usize].clone()));
        }

        if d > 0 {
            if k > prev_k {
                // Move Right (Remove from old)
                x -= 1;
                diffs.push(Diff::Remove(old[x as usize].clone()));
            } else {
                // Move Down (Add to new)
                y -= 1;
                diffs.push(Diff::Add(new[y as usize].clone()));
            }
            k = prev_k;
        }
    }

    diffs.reverse();
    diffs
}

#[cfg(test)]
mod tests {
    use super::{Diff, calculate_diff};

    #[test]
    fn test_empty_sequences() {
        let old: &[char] = &[];
        let new: &[char] = &[];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![]);
    }

    #[test]
    fn test_identical_sequences() {
        let old = &['A', 'B', 'C'];
        let new = &['A', 'B', 'C'];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![Diff::Common('A'), Diff::Common('B'), Diff::Common('C'),]
        );
    }

    #[test]
    fn test_all_additions() {
        let old: &[char] = &[];
        let new = &['A', 'B', 'C'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![Diff::Add('A'), Diff::Add('B'), Diff::Add('C'),]);
    }

    #[test]
    fn test_all_deletions() {
        let old = &['A', 'B', 'C'];
        let new: &[char] = &[];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![Diff::Remove('A'), Diff::Remove('B'), Diff::Remove('C'),]
        );
    }

    #[test]
    fn test_mixed_changes() {
        let old = &['A', 'B', 'C', 'D', 'F'];
        let new = &['A', 'C', 'D', 'E', 'F'];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![
                Diff::Common('A'),
                Diff::Remove('B'),
                Diff::Common('C'),
                Diff::Common('D'),
                Diff::Add('E'),
                Diff::Common('F'),
            ]
        );
    }

    #[test]
    fn test_replace() {
        let old = &['A', 'B', 'C'];
        let new = &['A', 'X', 'C'];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![
                Diff::Common('A'),
                Diff::Remove('B'),
                Diff::Add('X'),
                Diff::Common('C'),
            ]
        );
    }

    #[test]
    fn test_long_common_prefix() {
        let old = &['A', 'B', 'C', 'D', 'E'];
        let new = &['A', 'B', 'C', 'X', 'Y'];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![
                Diff::Common('A'),
                Diff::Common('B'),
                Diff::Common('C'),
                Diff::Remove('D'),
                Diff::Remove('E'),
                Diff::Add('X'),
                Diff::Add('Y'),
            ]
        );
    }

    #[test]
    fn test_long_common_suffix() {
        let old = &['A', 'B', 'C', 'D', 'E'];
        let new = &['X', 'Y', 'C', 'D', 'E'];
        let diff = calculate_diff(old, new);
        assert_eq!(
            diff,
            vec![
                Diff::Remove('A'),
                Diff::Remove('B'),
                Diff::Add('X'),
                Diff::Add('Y'),
                Diff::Common('C'),
                Diff::Common('D'),
                Diff::Common('E'),
            ]
        );
    }
}
