use dsp::block::{Adapter, BlockOutput};
use num::Complex;
use std::mem::transmute;

mod fftwsys;

pub struct DFTC64 {
    plan:       fftwsys::fftw_plan,
    bins:       i32,
}

impl DFTC64 {
    pub fn new(bins: i32, forward: bool) -> DFTC64 {
        let plan;
        unsafe {
            // Since we are doing FFTW_PATIENT we need to provide it
            // with a temporary buffer so it can do some testing to
            // optimize itself to give us the best performance.
            //
            // I need to save this and use it across all instances?
            // Since I belieive this can be done?
            //
            let mut tmp: Vec<Complex<f64>> = Vec::new();
            for _ in 0..bins {
                tmp.push(Complex::new(0.0, 0.0));
            }
            plan = fftwsys::fftw_plan_dft_1d(
                bins as i32,
                transmute(tmp.as_slice().as_ptr()),
                transmute(tmp.as_slice().as_ptr()),
                if forward { fftwsys::FFTW_FORWARD } else { fftwsys::FFTW_BACKWARD },
                fftwsys::FFTW_PATIENT,
            );
        }
        DFTC64 {
            bins:   bins,
            plan:   plan,
        }        
    }

    /// A helper function to allow you to easily get the frequency for each bin.
    ///
    /// `cfreq`: center frequency of the domain
    /// `sps`: samples per second
    /// `x`: index of the bin (0..bins-1)
    ///
    /// You can pass `0.0` for `cfreq` since this is simply added to the result.
    pub fn freqofbin(&self, cfreq: f64, sps: f64, x: i32) -> f64 {
        if x <= self.bins / 2 {
            cfreq + x as f64 * sps / self.bins as f64
        } else {
            cfreq - (self.bins - x) as f64 * sps / self.bins as f64
        }
    }   
}
 
impl Drop for DFTC64 {
    fn drop(&mut self) {
        unsafe {
            fftwsys::fftw_destroy_plan(self.plan);
        }
    }
}

impl Adapter<Complex<f64>, Complex<f64>> for DFTC64 {
    fn work(&mut self, mut input: Vec<Complex<f64>>) -> BlockOutput<Complex<f64>> {
        if input.len() != self.bins as usize {
            // I could have built-in a chunker, but I decided to side with 
            // performance. If the library user wishes to provide arbitrary
            // sized input, then they should explicitly use a chunker block.
            return BlockOutput::ErrorInputSizeInvalid(format!("The input size must be {} for performance! See, dsp::block::Chunker for a solution!", self.bins));
        }
        unsafe {
            fftwsys::fftw_execute_dft(
                self.plan, 
                transmute(input.as_slice().as_ptr()), 
                transmute(input.as_slice().as_ptr())
            );
        }
        BlockOutput::Ready(input)
    }
}