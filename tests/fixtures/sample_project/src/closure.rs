// Closure inside outer fn — closure should NOT be extracted as a separate
// Function (verified by parser tests / fixture-level integration).

pub fn run_with_closure(items: &[i32]) -> i32 {
    let total: i32 = items.iter().map(|x| x * 2).sum();
    total
}
