#![cfg(test)]
use core_lib::core;

#[test]
fn test_id_generation() {
    let res = core::id::short_id();
    println!("generate id => {}", res);
    assert_eq!(res.len(), 12);
}
