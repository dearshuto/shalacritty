
#[derive(Debug, PartialEq, Clone)]
pub enum Diff<T> {
    Common(T),
    Add(T),
    Remove(T),
}

pub fn calculate_diff<T: PartialEq + Clone>(
    old: &[T],
    new: &[T],
) -> Vec<Diff<T>> {
    let n = old.len(); // Length of old sequence
    let m = new.len(); // Length of new sequence

    // If both sequences are empty, return an empty diff
    if n == 0 && m == 0 {
        return Vec::new();
    }

    // `max_d` is the maximum possible edit distance, which is n + m
    let max_d = n + m;
    // `offset` is used to map k (which can be negative) to a non-negative array index
    // k ranges from -(n+m) to n+m.
    // Index: k + offset => 0 to 2*(n+m)
    let offset = max_d;

    // `v` array: stores the furthest reaching x-coordinate for each diagonal `k`
    // `v[k + offset]` corresponds to diagonal `k`
    let mut v = vec![0; 2 * max_d + 1];

    // `path_history` stores the state of `v` *before* each `d` iteration.
    // `path_history[d]` will be `v` before `d`-th iteration starts.
    let mut path_history: Vec<Vec<usize>> = Vec::new();

    for d in 0..=max_d {
        // Store the current state of `v` for backtracking
        path_history.push(v.clone());

        // Iterate through possible diagonals `k` for the current `d`
        // `k` ranges from -d to d, in steps of 2
        for k_raw in (-d as isize)..=(d as isize) {
            // `k_raw` is the diagonal index (x - y)

            // Skip diagonals that are out of bounds for the grid
            // x (index in old) is from 0 to n. y (index in new) is from 0 to m.
            // k = x - y. So min k = 0 - m = -m. Max k = n - 0 = n.
            if k_raw < -(m as isize) || k_raw > (n as isize) {
                continue;
            }

            let k_idx = (k_raw + offset) as usize;

            let mut x; // x-coordinate (index in old sequence)

            // Determine whether to move down (delete from old) or right (insert into new)
            // This is based on which path segment (from k-1 or k+1) has reached further
            // `v[k_idx - 1]` is the furthest x reached on diagonal `k-1`
            // `v[k_idx + 1]` is the furthest x reached on diagonal `k+1`
            // If `k_raw == -d`, we must have come from `k_raw + 1` (insert, effectively).
            // If `k_raw == d`, we must have come from `k_raw - 1` (delete, effectively).
            if k_raw == -(d as isize) || (k_raw != (d as isize) && v[k_idx - 1] < v[k_idx + 1]) {
                // If this condition is true, we came from diagonal `k_raw + 1`.
                // This corresponds to a deletion in the `old` sequence (moving down on the grid).
                x = v[k_idx + 1];
            } else {
                // If this condition is false, we came from diagonal `k_raw - 1`.
                // This corresponds to an insertion in the `new` sequence (moving right on the grid).
                x = v[k_idx - 1] + 1;
            }

            // Calculate the corresponding y-coordinate
            let mut y = x as isize - k_raw;

            // Follow common elements (diagonal moves)
            while (x as usize) < n && (y as usize) < m && old[x as usize] == new[y as usize] {
                x += 1;
                y += 1;
            }

            // Update the furthest reaching x-coordinate for diagonal `k_raw`
            v[k_idx] = x;

            // If the end of both sequences is reached, reconstruct the path
            if (x as usize) >= n && (y as usize) >= m {
                return reconstruct_path(old, new, &path_history, d, k_raw, offset);
            }
        }
    }

    // This part should ideally not be reached if there is always a path (which there should be).
    Vec::new()
}

// Helper function to reconstruct the path from `path_history`
fn reconstruct_path<T: PartialEq + Clone>(
    old: &[T],
    new: &[T],
    path_history: &[Vec<usize>],
    d_final: usize,
    mut k: isize, // The final diagonal k
    offset: usize,
) -> Vec<Diff<T>> {
    let mut diffs = Vec::new();
    let mut x = old.len() as isize; // Start from the end of the old sequence
    let mut y = new.len() as isize; // Start from the end of the new sequence

    // Iterate backwards from the final `d` down to 1
    for d in (1..=d_final).rev() {
        // `v_prev_d` is the state of `v` *before* the `d`-th iteration.
        // We use this to figure out where we came from at `d-1`.
        let v_prev_d = &path_history[d];

        let k_idx = (k + offset) as usize;

        // Determine if the current position (x,y) at depth `d` was reached by:
        // 1. A deletion: from (x-1, y) on diagonal `k+1` (k decreases by 1 to reach k_raw)
        // 2. An insertion: from (x, y-1) on diagonal `k-1` (k increases by 1 to reach k_raw)

        // Compare the x-coordinates from the two possible previous diagonals at depth `d-1`.
        // The values in `v_prev_d` correspond to the `v` array *before* the current `d` iteration.

        // x from path where old[x-1] was deleted (came from k+1 diagonal at previous d)
        // This means at depth d-1, on diagonal k+1, the furthest x was v_prev_d[k+1 + offset]
        // And the step taken was from (x_delete, y_delete) to (x_delete+1, y_delete).
        // So, x = v_prev_d[k+1 + offset] (if came from k+1)
        let x_from_delete_path = v_prev_d[(k + 1 + offset) as usize];

        // x from path where new[y-1] was inserted (came from k-1 diagonal at previous d)
        // This means at depth d-1, on diagonal k-1, the furthest x was v_prev_d[k-1 + offset]
        // And the step taken was from (x_insert, y_insert) to (x_insert, y_insert+1).
        // So, x = v_prev_d[k-1 + offset] + 1 (if came from k-1)
        let x_from_insert_path = v_prev_d[(k - 1 + offset) as usize] + 1;

        let is_deletion_move; // True if the last move was a deletion (old[x-1] was removed)

        // The logic for choosing the previous step needs to mirror the forward step.
        // If k == -d or (k != d and current_v_for_k_minus_1 < current_v_for_k_plus_1) then delete.
        // The "d" here is the *current* d we are backtracking from.
        if k == -(d as isize) || (k != (d as isize) && x_from_insert_path < x_from_delete_path) {
            is_deletion_move = true; // The step *was* a deletion from old (came from k+1)
        } else {
            is_deletion_move = false; // The step *was* an insertion into new (came from k-1)
        }

        let actual_prev_k;
        let actual_prev_x;
        let actual_prev_y;

        if is_deletion_move {
            // If the move was a deletion, we came from diagonal `k+1` at depth `d-1`.
            actual_prev_k = k + 1;
            actual_prev_x = v_prev_d[(actual_prev_k + offset) as usize];
        } else {
            // If the move was an insertion, we came from diagonal `k-1` at depth `d-1`.
            actual_prev_k = k - 1;
            actual_prev_x = v_prev_d[(actual_prev_k + offset) as usize] + 1;
        }
        actual_prev_y = actual_prev_x as isize - actual_prev_k;

        // Trace back common elements (diagonal moves in reverse)
        // These are the common elements found *before* the last diff step.
        while x > actual_prev_x && y > actual_prev_y && old[(x - 1) as usize] == new[(y - 1) as usize] {
            x -= 1;
            y -= 1;
            diffs.push(Diff::Common(old[x as usize].clone()));
        }

        // Add the diff element that corresponds to the `is_deletion_move`
        if is_deletion_move {
            diffs.push(Diff::Remove(old[(x - 1) as usize].clone()));
            x -= 1;
        } else {
            diffs.push(Diff::Add(new[(y - 1) as usize].clone()));
            y -= 1;
        }

        // Update `k` for the next iteration (i.e., for depth `d-1`)
        k = actual_prev_k;
    }

    // After the loop, `x` and `y` might not be zero if there were common elements
    // leading up to the very first `d` step.
    // Handle any remaining common elements from (0,0) to the start of the first diff.
    while x > 0 && y > 0 && old[(x - 1) as usize] == new[(y - 1) as usize] {
        x -= 1;
        y -= 1;
        diffs.push(Diff::Common(old[x as usize].clone()));
    }

    diffs.reverse(); // Reverse the diffs to get them in the correct order
    diffs
}

#[cfg(test)]
mod tests {
    use super::{calculate_diff, Diff};

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
        assert_eq!(diff, vec![
            Diff::Common('A'),
            Diff::Common('B'),
            Diff::Common('C'),
        ]);
    }

    #[test]
    fn test_all_additions() {
        let old: &[char] = &[];
        let new = &['A', 'B', 'C'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Add('A'),
            Diff::Add('B'),
            Diff::Add('C'),
        ]);
    }

    #[test]
    fn test_all_deletions() {
        let old = &['A', 'B', 'C'];
        let new: &[char] = &[];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Remove('A'),
            Diff::Remove('B'),
            Diff::Remove('C'),
        ]);
    }

    #[test]
    fn test_mixed_changes() {
        let old = &['A', 'B', 'C', 'D', 'F'];
        let new = &['A', 'C', 'D', 'E', 'F'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Common('A'),
            Diff::Remove('B'),
            Diff::Common('C'),
            Diff::Common('D'),
            Diff::Add('E'),
            Diff::Common('F'),
        ]);
    }

    #[test]
    fn test_replace() {
        let old = &['A', 'B', 'C'];
        let new = &['A', 'X', 'C'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Common('A'),
            Diff::Remove('B'),
            Diff::Add('X'),
            Diff::Common('C'),
        ]);
    }

    #[test]
    fn test_long_common_prefix() {
        let old = &['A', 'B', 'C', 'D', 'E'];
        let new = &['A', 'B', 'C', 'X', 'Y'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Common('A'),
            Diff::Common('B'),
            Diff::Common('C'),
            Diff::Remove('D'),
            Diff::Remove('E'),
            Diff::Add('X'),
            Diff::Add('Y'),
        ]);
    }

    #[test]
    fn test_long_common_suffix() {
        let old = &['A', 'B', 'C', 'D', 'E'];
        let new = &['X', 'Y', 'C', 'D', 'E'];
        let diff = calculate_diff(old, new);
        assert_eq!(diff, vec![
            Diff::Remove('A'),
            Diff::Remove('B'),
            Diff::Add('X'),
            Diff::Add('Y'),
            Diff::Common('C'),
            Diff::Common('D'),
            Diff::Common('E'),
        ]);
    }
}
