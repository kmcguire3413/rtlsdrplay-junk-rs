#![feature(core)]

fn main() {
    let mut a: Vec<u8> = Vec::with_capacity(1024 * 1024 * 100);
    unsafe { a.set_len(1024 * 1024 * 100); }

    let mut sum: u8 = 0;
    for c in 0..1000 {
        sum = sum.wrapping_add(fast(a.as_slice()));
    }

    println!("{}", sum);
}

fn slow(a: &[u8]) -> u8 {
    let mut sum = 0u8;
    for ndx in 0..a.len() {
        sum = sum.wrapping_add(a[ndx]);
    }

    sum
}

fn fast(a: &[u8]) -> u8 {
    let mut sum = 0u8;
    let mut ndx = 0;
    for s in a {
        sum = sum.wrapping_add(*s);
        ndx += sum;
    }

    sum.wrapping_add(ndx as u8)
}