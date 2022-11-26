pub struct LifeState {
    pub matrix: [[bool; 5]; 5],
}

impl LifeState {
    pub fn next_state(&mut self) {
        let mut next_state_matrix = [[false; 5]; 5];

        for (row_n, row) in self.matrix.into_iter().enumerate() {
            for (col_n, element) in row.into_iter().enumerate() {
                let n_neighbors = count_live_neighbors(self.matrix, row_n, col_n);

                next_state_matrix[row_n][col_n] = match (element, n_neighbors) {
                    // Cell alive with 2 or 3 neighbors:
                    (true, 2 | 3) => true,
                    // Cell dead with 3 neighbors:
                    (false, 3) => true,
                    // Any other case:
                    _ => false,
                };
            }
        }
        self.matrix = next_state_matrix;
    }
    pub fn int_matrix(&self) -> [[u8; 5]; 5] {
        // To display the matrix using the LEDs, it must be converted to u8.
        self.matrix.map(|row| row.map(|element| element as u8))
    }
}

fn count_live_neighbors(matrix: [[bool; 5]; 5], target_row: usize, target_col: usize) -> u8 {
    // Compute the number of live neighbors that the element row, column of the matrix
    // matrix has. Live neighbor are the ones set to true.

    // To avoid having to deal with the special cases of the edges of the matrix, a new
    // the 5x5 matrix passed to the function is padded with false values to generate
    // a new 7x7 matrix. We can then operate on this new matrix knowing that the element
    // to study is never going to be on the edge.

    let mut padded_matrix: [[bool; 7]; 7] = [[false; 7]; 7];

    for (row_n, row) in matrix.into_iter().enumerate() {
        for (col_n, element) in row.into_iter().enumerate() {
            padded_matrix[row_n + 1][col_n + 1] = element;
        }
    }

    // Indexes of the target element on the new matrix:
    let new_target_row = target_row + 1;
    let new_target_col = target_col + 1;

    let neighbors = [
        // Neighbors on top:
        (new_target_row - 1, new_target_col - 1),
        (new_target_row - 1, new_target_col),
        (new_target_row - 1, new_target_col + 1),
        // Neighbors on the side:
        (new_target_row, new_target_col - 1),
        (new_target_row, new_target_col + 1),
        // Neighbors bellow:
        (new_target_row + 1, new_target_col - 1),
        (new_target_row + 1, new_target_col),
        (new_target_row + 1, new_target_col + 1),
    ];

    let mut n_live_neighbors = 0;
    for (i, j) in neighbors.into_iter() {
        if padded_matrix[i][j] {
            n_live_neighbors += 1;
        }
    }
    n_live_neighbors
}
