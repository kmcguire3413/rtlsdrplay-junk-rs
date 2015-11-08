//! This provides demodulation blocks for FM. 
//!
//! I hope the blocks here may vary 
//! using different techniques. Some may be fast, accurate, flexible, and have 
//! various cons and pros. This will provide you with a demodulator block that 
//! molds to your needs, while conserving system resources.
use dsp::block::{Adapter, BlockOutput};
use num::Complex;
use std::f64::consts::PI;


pub struct FastAtan {
    tbl:        Vec<f64>,
    steps:      usize,
}

impl FastAtan {
    pub fn new(steps: usize) -> FastAtan {
        let mut tbl: Vec<f64> = Vec::new();
        // produce table for value -1.0 to 1.0
        for step in 0..steps {
            tbl.push(((step / steps) as f64 * 2.0 - 1.0).atan());
        }
        FastAtan {
            tbl:    tbl,
            steps:  steps,
        }
    }

    pub fn atan(&self, v: f64) -> f64 {
        // make it 0.0 to 2.0
        println!("v:{}", v);
        let fndx = ((v + 1.0) / 2.0) * self.steps as f64;
        let fdif = fndx - fndx.floor();
        let ndx = fndx as usize;
        self.tbl[ndx]
    }
}

/// A crude but working FM demodulator. I hope this is easy to learn from.
pub struct CrudeC64I8 {
    fatan:          FastAtan,
}

impl CrudeC64I8 {
    pub fn new() -> CrudeC64I8 {
        CrudeC64I8 {
            fatan:      FastAtan::new(256),
        }
    }
}

impl Adapter<Complex<f64>, i8> for CrudeC64I8 {
    fn work(&mut self, input: Vec<Complex<f64>>) -> BlockOutput<i8> {
        let mut out: Vec<i8> = Vec::new();
        for s in input.windows(2) {
            // this works
            //let a = s[0].im.atan2(s[0].re);
            //let b = s[1].im.atan2(s[1].re);
            // -PI to 0.0 to PI
            //let c = b - a;

            let a = s[0].im / s[0].re;
            let b = s[1].im / s[1].re;
            let c = (b - a).atan();
            //let c = self.fatan.atan(b - a);


            // positive angle - positive deviation
            // negative angle - negative deviation

            // ratio..  -1.0 to 0.0 to 1.0
            let r = c / PI;
            //let th = 1.2;
            //let r = if r > th { th } else { r };
            //let r = if r < -th { -th } else { r };
            // convert to full swing
            out.push((128.0 * r) as i8);
        }
        BlockOutput::Ready(out)
    }
}

