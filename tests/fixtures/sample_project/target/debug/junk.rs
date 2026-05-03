// Lives under target/ — must be excluded by walker. Contains a function that
// would otherwise breach cognitive threshold; if it appears in scan output,
// the exclusion regressed.

pub fn junk_complex(x: i32) -> i32 {
    let mut r = 0;
    if x > 1 {
        if x > 2 {
            if x > 3 {
                if x > 4 {
                    if x > 5 {
                        r = x;
                    }
                }
            }
        }
    }
    r
}
