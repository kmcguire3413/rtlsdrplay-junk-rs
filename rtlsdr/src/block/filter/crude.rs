///! This contains some very crude, but usable filters. They claim neither
///! efficiency, accuracy, nor glory. Use them at your own peril!
use dsp::block::{Adapter, BlockOutput};
use num::Complex;


// single pole IIR for real signal
// Y= (a*X) + (1.0-a)*Y

pub struct LowF64 {
    last:       f64,
    scale:      f64,
}

pub struct HighF64 {
    last:       f64,
    scale:      f64,
}

pub struct LowC64 {
    i:      LowF64,
    q:      LowF64,
}

pub struct HighC64 {
    i:      HighF64,
    q:      HighF64,
}

impl LowF64 {
    pub fn new(scale: f64) -> LowF64 { LowF64 { last: 0.0, scale: scale } }
    #[inline]
    pub fn single(&mut self, mut sample: f64) -> f64 {
        let delta = sample - self.last;
        sample -= delta * self.scale;
        self.last = sample;
        sample
    }
}

impl HighF64 {
    pub fn new(scale: f64) -> HighF64 { HighF64 { last: 0.0, scale: scale } }
    #[inline]
    pub fn single(&mut self, sample: f64) -> f64 {
        let delta = sample - self.last;
        self.last = sample;
        delta * self.scale
    }
}

impl LowC64 {
    pub fn new(scale: f64) -> LowC64 { LowC64 { i: LowF64::new(scale), q: LowF64::new(scale) } }
}

impl HighC64 {
    pub fn new(scale: f64) -> HighC64 { HighC64 { i: HighF64::new(scale), q: HighF64::new(scale) } }
}

impl Adapter<f64, f64> for LowF64 {
    fn work(&mut self, mut input: Vec<f64>) -> BlockOutput<f64> {
        for sample in input.iter_mut() {
            *sample = self.single(*sample);
        }
        BlockOutput::Ready(input)
    }
}

impl Adapter<Complex<f64>, Complex<f64>> for LowC64 {
    fn work(&mut self, mut input: Vec<Complex<f64>>) -> BlockOutput<Complex<f64>> {
        for sample in input.iter_mut() {
            sample.re = self.i.single(sample.re);
            sample.im = self.q.single(sample.im);
        }
        BlockOutput::Ready(input)
    }
}

impl Adapter<Complex<f64>, Complex<f64>> for HighC64 {
    fn work(&mut self, mut input: Vec<Complex<f64>>) -> BlockOutput<Complex<f64>> {
        for sample in input.iter_mut() {
            sample.re = self.i.single(sample.re);
            sample.im = self.q.single(sample.im);
        }
        BlockOutput::Ready(input)
    }
}

impl Adapter<f64, f64> for HighF64 {
    fn work(&mut self, mut input: Vec<f64>) -> BlockOutput<f64> {
        for sample in input.iter_mut() {
            *sample = self.single(*sample);
        }
        BlockOutput::Ready(input)
    }
}