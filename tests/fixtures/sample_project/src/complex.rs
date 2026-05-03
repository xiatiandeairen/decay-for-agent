// Intentionally exceeds cognitive threshold (≥ 16) via deeply nested if-else
// chains plus a multi-arm match plus compound boolean conditions.

pub fn complex_logic(x: i32) -> i32 {
    let mut r = 0;
    if x > 10 {
        if x > 20 {
            if x > 30 {
                if x > 40 {
                    r += x;
                }
            }
        }
    }
    match x {
        1 => r += 1,
        2 => r += 2,
        3 => r += 3,
        _ => r += 0,
    }
    if x % 2 == 0 && x % 3 == 0 {
        r += 100;
    }
    r
}
