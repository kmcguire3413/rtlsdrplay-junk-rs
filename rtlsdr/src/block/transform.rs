//! This module transforms input chunks.
//!
//! It can do the following:
//!
//! * converts from one type to another
//! * does things not found under other modules
use std::vec::Vec;
use num::Complex;
use dsp::block::{BlockOutput, Adapter};

pub struct ConvC8C64;

/// Convert a signal of type Complex<i8> to Complex<f64> of range -1.0 to 1.0.
///
/// The conversion is done by `(real as f64 / 128.0, imaginary as f64 / 128.0)`.
///
/// _This allocates a new buffer on each invocation of `work`. I would like to
/// work on a better version that can support using a provided buffer, but I
/// will save doing that for later._
impl ConvC8C64 {
    pub fn new() -> ConvC8C64 {
        ConvC8C64
    }
}

impl Adapter<Complex<i8>, Complex<f64>> for ConvC8C64 {
    fn work(&mut self, input: Vec<Complex<i8>>) -> BlockOutput<Complex<f64>> {
        let mut out: Vec<Complex<f64>> = Vec::new();
        for sample in input.iter() {
            out.push(Complex::new(
                sample.re as f64 / 128.0,
                sample.im as f64 / 128.0,
            ));
        }
        BlockOutput::Ready(out)
    }
}