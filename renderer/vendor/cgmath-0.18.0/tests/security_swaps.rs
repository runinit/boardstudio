extern crate cgmath;

use cgmath::{Matrix, Matrix2, Matrix3, Matrix4, SquareMatrix};

macro_rules! swap_cases {
    ($name:ident, $matrix:ident, $size:expr) => {
        #[test]
        fn $name() {
            let mut original = $matrix::<f32>::identity();
            for column in 0..$size {
                for row in 0..$size {
                    original[column][row] = (column * $size + row) as f32;
                }
            }
            for a in 0..$size {
                for b in 0..$size {
                    let mut matrix = original;
                    matrix.swap_columns(a, b);
                    assert_eq!(matrix[a], original[b]);
                    assert_eq!(matrix[b], original[a]);
                    matrix.swap_columns(a, b);
                    assert_eq!(matrix, original);

                    for ar in 0..$size {
                        for br in 0..$size {
                            let mut matrix = original;
                            matrix.swap_elements((a, ar), (b, br));
                            assert_eq!(matrix[a][ar], original[b][br]);
                            assert_eq!(matrix[b][br], original[a][ar]);
                            matrix.swap_elements((a, ar), (b, br));
                            assert_eq!(matrix, original);
                        }
                    }
                }
            }
            for (a, b) in [(0, $size), ($size, 0), ($size, $size)] {
                assert!(std::panic::catch_unwind(|| {
                    let mut matrix = original;
                    matrix.swap_columns(a, b);
                }).is_err());
                assert!(std::panic::catch_unwind(|| {
                    let mut matrix = original;
                    matrix.swap_elements((a, 0), (b, 0));
                }).is_err());
                assert!(std::panic::catch_unwind(|| {
                    let mut matrix = original;
                    matrix.swap_elements((0, a), (0, b));
                }).is_err());
            }
        }
    };
}

swap_cases!(matrix2_swaps, Matrix2, 2);
swap_cases!(matrix3_swaps, Matrix3, 3);
swap_cases!(matrix4_swaps, Matrix4, 4);
