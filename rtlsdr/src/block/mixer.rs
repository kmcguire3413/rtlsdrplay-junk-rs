#![allow(dead_code)]
//use std::marker::PhantomData;
use std::num::Float;
use std::f64::consts::PI;
use dsp::block::{BlockOutput, Adapter};
use num::Complex;

/*
        let rpt = 1.0 / sps as f64;

        for x in 0..dsignal.len() {
            let rad = (rpt * x as f64 * PI * 2.0 * (942200.0 - 29459.0));
            let p = Complex::new(rad.cos(), rad.sin());
            dsignal[x] = p * dsignal[x];
        }
*/

/// Sinsodial mixer.
///
/// - Can be used to rotate/shift in the frequency domain.
/// - Inject sinsodial signal.
pub struct Sinsodial {
    sps:            f64,
    tps:            f64,
    ndx:            u64,
    freq:           f64,
    sphase:         f64,
    amp:            f64,
}

impl Sinsodial {
    pub fn new(sps: f64, freq: f64, amp: f64, phase: f64) -> Sinsodial {
        Sinsodial {
            tps:        1.0 / sps,
            sps:        sps,
            ndx:        0,
            freq:       freq,
            sphase:     phase,
            amp:        amp,
        }
    }
}

impl Adapter<Complex<f64>, Complex<f64>> for Sinsodial {
    fn work(&mut self, mut input: Vec<Complex<f64>>) -> BlockOutput<Complex<f64>> {
        for sample in input.iter_mut() {
            let rad = self.tps * self.ndx as f64 * PI * 2.0 * self.freq;
            *sample = Complex::new(rad.cos(), rad.sin()) * *sample;
            self.ndx += 1;
        }
        BlockOutput::Ready(input)
    }
}