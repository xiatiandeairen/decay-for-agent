// Intentionally exceeds nesting threshold (≥ 5).

pub fn deeply_nested(x: i32) -> i32 {
    let mut r = 0;
    if x > 0 {
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
    }
    r
}
