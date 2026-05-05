use decay::pipeline;
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let arg = std::env::args().nth(1).expect("usage: find_dupes <root>");
    let root = Path::new(&arg);
    let funcs = pipeline::scan(root).expect("scan");
    println!("scanned {} functions", funcs.len());

    let mut by_hash: HashMap<u64, Vec<&decay::types::Function>> = HashMap::new();
    for f in &funcs {
        by_hash.entry(f.signature_hash).or_default().push(f);
    }
    let mut dups: Vec<_> = by_hash.iter().filter(|(_, v)| v.len() > 1).collect();
    dups.sort_by_key(|(_, v)| -(v.len() as i64));

    println!("colliding hash groups: {}", dups.len());
    for (h, v) in dups.iter().take(10) {
        println!("  hash {:#x}  ({} fns)", h, v.len());
        for f in v.iter().take(5) {
            println!(
                "    {}:{}  impl_context={:?}  fn {}  params={:?}",
                f.file, f.start_line, f.impl_context, f.name, f.param_types
            );
        }
    }
}
