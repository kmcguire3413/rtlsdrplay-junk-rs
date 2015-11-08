#![feature(collections)]
#![feature(convert)]
#![feature(libc)]
#![feature(core)]
extern crate libc;
extern crate num;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem::{zeroed, transmute};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use num::Complex;

use std::num::Float;
use std::f64::consts::PI;

pub mod dsp;

/*
/// Perform the Goertzel filter on a stream of IQ floating-point data,
/// returning the total power, and derivable phase of the signal.
fn goertzeliq(sps: u32, freq: f32, iqstream: &[f32]) -> (f32, f32) {
    let ns = iqstream.len() as f32;
    let w = (2.0 * PI as f32 * freq) / ns;
    let coeff = (2.0 * w.cos(), 2.0 * w.sin());
    let mut q2 = (0.0, 0.0);
    let mut q1 = (0.0, 0.0);

    for x in 0..iqstream.len() / 2 {
        let c = (iqstream[x*2+0], iqstream[x*2+1]); 
        let q0 = (
            c.0 + q1.0 * coeff.0 - q2.0,
            c.1 + q1.1 * coeff.1 - q2.1
        );
        q2 = q1;
        q1 = q0;
    }

    let power = (
        q2.0*q2.0 + q1.0*q1.0 - coeff.0*q1.0*q2.0,
        q2.1*q2.1 + q1.1*q1.1 - coeff.1*q1.1*q2.1,
    );

    (power.0, power.1)
}

/// Get the difference between two angles scaled between 0.0 and 2*PI.
fn pianglediff(a: f64, b: f64) -> f64 {
    // Get one of the two distances, from angle to angle.
    let da = (a - b).abs();
    // If larger than PI then it is the largest distance.
    if da > PI { 
        // Correct the distance.
        PI - (da - PI)
    } else {
        // We have the smallest distance.
        da
    }
}

fn getfftatfreqfortime(rtlsdr: &RTLSDR, cfreq: u32, fftbins: usize, flush: usize, count: usize) -> Vec<Complex<f64>> {
    let mut mx: Vec<Complex<f64>> = Vec::with_capacity(fftbins);

    let fftplan = unsafe { fftwsys::fftw_plan_dft_1d(
        fftbins as i32,
        std::mem::transmute(0 as usize),
        std::mem::transmute(0 as usize),
        fftwsys::FFTW_FORWARD,
        fftwsys::FFTW_ESTIMATE
    )};

    let fft = getfftatfreq(rtlsdr, cfreq, fftbins, flush, Option::Some(fftplan));
    for bin in fft.iter() {
        mx.push(*bin);
    }

    for cc in 1..count {
        let fft = getfftatfreq(rtlsdr, cfreq, fftbins, 0, Option::Some(fftplan));
        // Summation
        for i in 0..fft.len() {
            if fft[i].norm_sqr() > mx[i].norm_sqr() {
                mx[i] = fft[i];
            }
        }
    }

    unsafe {
        fftwsys::fftw_destroy_plan(fftplan);
    }

    mx
}

fn smoother(signal: &mut Vec<Complex<f64>>, dir: isize, off: usize, len: usize) {
    for x in 0..len {
        let cur = (x as isize * dir + off as isize) as usize;
        signal[cur] = signal[cur].scale(x as f64 / len as f64);
    }
}

struct SIFilter {
    last:       f64,
    scale:      f64,
}

impl SIFilter {
    fn new(scale: f64) -> SIFilter { SIFilter { last: 0.0, scale: scale } }
    fn low(&mut self, mut sample: f64) -> f64 {
        let delta = sample - self.last;
        sample -= delta * self.scale;
        self.last = sample;
        sample
    }
    fn high(&mut self, mut sample: f64) -> f64 {
        let delta = sample - self.last;
        self.last = sample;
        delta * self.scale
    }
}

struct SIFilterC {
    i:      SIFilter,
    q:      SIFilter,
}

impl SIFilterC {
    fn new(scale: f64) -> SIFilterC { SIFilterC { i: SIFilter::new(scale), q: SIFilter::new(scale) } }
    fn high(&mut self, mut sample: &Complex<f64>) -> Complex<f64> {
        Complex::new(self.i.high(sample.re), self.q.high(sample.im))
    }
    fn low(&mut self, mut sample: &Complex<f64>) -> Complex<f64> {
        Complex::new(self.i.low(sample.re), self.q.low(sample.im))
    }        
}

fn getfftatfreq(rtlsdr: &RTLSDR, cfreq: u32, fftbins: usize, flush: usize, plan: Option<fftwsys::fftwl_plan>) -> Vec<Complex<f64>> {
    rtlsdr.setcenterfreq(cfreq);
    let sps = rtlsdr.getsamplerate();
    let mut signal: Vec<Complex<f64>>;
    {
        if flush > 0 {
            rtlsdr.syncread(flush as isize);
        }
        // We are reading bytes and we need two bytes per sample, and
        // each run of the FFT is only going to make use of a certain
        // number. So lets just read exactly what we need.
        let mut out = rtlsdr.syncread((fftbins * 2) as isize);
        // Convert each value into a signed 64-bit floating point number.
        signal = Vec::with_capacity(fftbins as usize);
        for s in out.chunks(2) {
            signal.push(Complex::new(
                (s[0] as f64 - 128.0) / 128.0,
                (s[1] as f64 - 128.0) / 128.0
            ));
        }
    }

    // Remove DC component
    let mut sum = (0.0f64, 0.0f64);
    for x in signal.iter() {
        sum.0 += x.re;
        sum.1 += x.im;
    }

    let ave = (
        (sum.0 / signal.len() as f64) as f64,
        (sum.1 / signal.len() as f64) as f64,
    );

    for sample in signal.iter_mut() {
        sample.re -= ave.0;
        sample.im -= ave.1;
    }

    smoother(&mut signal, 1, 0, 10);
    let sz = signal.len();
    smoother(&mut signal, -1, sz - 1, 10);

    println!("creating file 'signal_fft'");
    let mut fd = File::create("signal_fft").unwrap();
    {
        // Perform the DFT.
        let mut fft: Vec<Complex<f64>> = Vec::with_capacity(fftbins);
        unsafe {
            fft.set_len(fftbins);

            let fftplan;
            if plan.is_none() {
                fftplan = fftwsys::fftw_plan_dft_1d(
                    fftbins as i32,
                    std::mem::transmute(signal.as_slice().as_ptr()),
                    std::mem::transmute(fft.as_slice().as_ptr()),
                    fftwsys::FFTW_FORWARD,
                    fftwsys::FFTW_ESTIMATE
                );
            } else {
                fftplan = plan.unwrap();
            }

            fftwsys::fftw_execute_dft(
                fftplan,
                std::mem::transmute(signal.as_slice().as_ptr()),
                std::mem::transmute(fft.as_slice().as_ptr())
            );

            if plan.is_none() {
                fftwsys::fftw_destroy_plan(fftplan);
            }
        }

        for x in 1..fft.len() {
            let freq = fftbintofreq(0.0, sps as f32, fftbins, x);
            fd.write(format!("{} {}\n", freq, fft[x].norm()).as_bytes());
        }
    }

    println!("creating file 'test'");
    let mut fd = File::create("test").unwrap();
        let mut dsignal = signal.clone();

        let mut charge = (0.0f64, 0.0f64);
        let capacity = 1.0;

        let rpt = 1.0 / sps as f64;

        for x in 0..dsignal.len() {
            let rad = (rpt * x as f64 * PI * 2.0 * (942200.0 - 29459.0));
            let p = Complex::new(rad.cos(), rad.sin());
            dsignal[x] = p * dsignal[x];
        }

        for _ in 0..20 {
            let mut filter = SIFilterC::new(0.3);
            for sample in dsignal.iter_mut() {
                *sample = filter.low(sample);
                //println!("sample:{:?}", sample);
                //sample.re = sample.re * 2.0;
                //sample.im = sample.im * 2.0;
            }
        }

        // Perform the DFT.
        let mut fft: Vec<Complex<f64>> = Vec::with_capacity(fftbins);
        unsafe {
            fft.set_len(fftbins);

            let fftplan;
            if plan.is_none() {
                fftplan = fftwsys::fftw_plan_dft_1d(
                    fftbins as i32,
                    std::mem::transmute(dsignal.as_slice().as_ptr()),
                    std::mem::transmute(fft.as_slice().as_ptr()),
                    fftwsys::FFTW_FORWARD,
                    fftwsys::FFTW_ESTIMATE
                );
            } else {
                fftplan = plan.unwrap();
            }

            fftwsys::fftw_execute_dft(
                fftplan,
                std::mem::transmute(dsignal.as_slice().as_ptr()),
                std::mem::transmute(fft.as_slice().as_ptr())
            );

            if plan.is_none() {
                fftwsys::fftw_destroy_plan(fftplan);
            }
        }

        for x in 1..fft.len() {
            let freq = fftbintofreq(0.0, sps as f32, fftbins, x);
            fd.write(format!("{} {}\n", freq, fft[x].norm()).as_bytes());
        }
    Vec::new()
}

fn fftbintofreq(cfreq: f32, sps: f32, fftlen: usize, x: usize) -> f32 {
    if x <= fftlen / 2 {
        cfreq + x as f32 * sps / fftlen as f32
    } else {
        cfreq - (fftlen - x) as f32 * sps / fftlen as f32
    }
}

struct FilterBandPass {
    taps:       Vec<f64>,  // m_taps
    sr:         Vec<f64>,  // m_sr
    lambda:     f64,    // m_lambda
    phi:        f64,    // m_phi
    sps:        f64,    // m_Fs
    low:        f64,    // m_Fx
    high:       f64,    // m_Fu
}

impl FilterBandPass {
    pub fn new(numtaps: usize, sps: f64, low: f64, high: f64) -> FilterBandPass {
        let mut fbp = FilterBandPass {
            taps:       Vec::new(),
            sr:         Vec::new(),
            lambda:     PI * low / (sps / 2.0),
            phi:        PI * high / (sps / 2.0),
            sps:        sps,
            low:        low,
            high:       high,
        };

        for n in 0..numtaps {
            let mm = n as f64 - (numtaps as f64 - 1.0) / 2.0;
            if mm == 0.0 {
                fbp.taps.push((fbp.phi - fbp.lambda) / PI);
            } else {
                fbp.taps.push(
                    ((mm * fbp.phi).sin() - (mm * fbp.lambda).sin()) / (mm * PI)
                );
            }
            fbp.sr.push(0.0);
        }

        fbp
    }

    pub fn work(&mut self, sample: f64) -> f64 {
        let mut i = self.taps.len() - 1;
        while i > 1 {
            self.sr[i] = self.sr[i - 1];
            i -= 1;
        }
        self.sr[0] = sample;

        let mut result: f64 = 0.0;
        for i in 0..self.taps.len() {
            result += self.sr[i] * self.taps[i];
        }

        result
    }
}

fn main() {
    let rtlsdr = RTLSDR::new(0, 0).unwrap();
    let cfreq = 98000000;
    //let cfreq = 145500000;
    let sps = rtlsdr.getsamplerate();

    println!("sps:{}", sps);

    // Get an FFT for about a second of time.
    //let fft = getfftatfreqfortime(&rtlsdr, cfreq, 4096, (sps * 4) as usize, 200);
    let fft = getfftatfreq(&rtlsdr, cfreq, 256, (sps * 4) as usize, Option::None);

    // freq = center_freq - (tb.usrp_rate / 2) + (tb.channel_bandwidth * i_bin)

    let mut f = File::create("tmp").unwrap();
    // Convert into slopes.
    for x in 1..fft.len() {
        let pwr = fft[x].norm();
        let freq = fftbintofreq(cfreq as f32, sps as f32, fft.len(), x);
        f.write(format!("{} {}\n", freq, pwr).as_bytes());
        //if pwr > 100 {
            //println!("signal")
        //}
    }


    //for x in 0..300 {
    //    println!("{} {} {}", x, fout[x*2+0], fout[x*2+1]);
    //}
    //if true {
    //    return;
    //}
}
*/

fn main() {
    use dsp::block::Source;
    use dsp::block::BlockOutput;
    use dsp::block::Adapter;
    use dsp::block::Sink;

    // Get the radio signal data.
    let mut source = dsp::block::source::rtlsdr::USB::new(0, 0).unwrap();
    println!("ZZ");
    let mut dft = dsp::block::fft::fftw::DFTC64::new(256, true);
    println!("YY");
    let mut convc8c64 = dsp::block::transform::ConvC8C64::new();
    println!("AA");
    let mut vfo0 = dsp::block::mixer::Sinsodial::new(
        source.getsamplerate() as f64, -167284.0, 1.0, 0.0
    );
    let mut vfo1 = dsp::block::mixer::Sinsodial::new(
        source.getsamplerate() as f64, 707899.0, 1.0, 0.0
    );
    println!("BB");
    // We need to apply the filter in layers to produce a sharp edge. So we
    // create multiple instances so we can iterate through them and apply them
    // to the signal. This is a very crude filter.
    let mut filterlow0: Vec<dsp::block::filter::crude::LowC64> = Vec::new();
    for _ in 0..1 {
        filterlow0.push(dsp::block::filter::crude::LowC64::new(0.8));
    }
    let mut filterlow1: Vec<dsp::block::filter::crude::LowC64> = Vec::new();
    for _ in 0..1 {
        filterlow1.push(dsp::block::filter::crude::LowC64::new(0.8));
    }
    let mut fm0 = dsp::block::demod::fm::CrudeC64I8::new();
    let mut fm1 = dsp::block::demod::fm::CrudeC64I8::new();
    let mut auds0 = dsp::block::sink::WaveFileI8::new(
        String::from_str("auds0.wav"), source.getsamplerate()
    );
    let mut auds1 = dsp::block::sink::WaveFileI8::new(
        String::from_str("auds1.wav"), source.getsamplerate()
    );

    // Here we create our pipeline which actually branches towards the end
    // into decoding two seperate FM signals into audio data. If we desired
    // we could create a thread and run our source in asynchronous mode and
    // process data in realtime while buffering the output somewhere.

    fn filefft(dft: &mut dsp::block::fft::fftw::DFTC64, path: &str, data: Vec<Complex<f64>>, sps: f64) {
        let fft: Vec<Complex<f64>> = dft.work(data).unwrap();
        // Dump this diagnostic data to disk so that we can take a peek.
        let mut fd = File::create(path).unwrap();
        for binndx in 0..fft.len() {
            let bin = fft[binndx];
            fd.write(format!("{} {}\n", 
                dft.freqofbin(
                    0.0,
                    sps,
                    binndx as i32
                ), bin.norm()
            ).as_bytes());
        }        
    }

    // Get some radio data and convert it to a more accurate representation for computation.
    println!("reading from rtlsdr dongle via usb");
    // This should really be SourceAmount::Exact(256), but I have not implemented 
    // that yet! However, I know that it will return 256, and if it does not then
    // the DFT below will panic the program.
    let sps = source.getsamplerate();
    let data: Vec<Complex<i8>> = source.read(dsp::block::SourceAmount::AtMost(sps as usize * 4)).unwrap();
    println!("converting from c8 to c64");
    let data: Vec<Complex<f64>> = convc8c64.work(data).unwrap();
    //filefft(&mut dft, "signal", data.clone(), sps as f64);
    // Use our VFO as a IF mixer and center on top of two different frequencies.
    println!("doing vfo0 mixing");
    let mut data0: Vec<Complex<f64>> = vfo0.work(data.clone()).unwrap();
    println!("doing vfo1 mixing");
    //let mut data1: Vec<Complex<f64>> = vfo1.work(data).unwrap();
    let mut data1 = data0.clone();
    // Filter each stream by applying multiple iterations of a crude filter.
    println!("filtering vfo0 output");
    for filter in filterlow0.iter_mut() {
        data0 = filter.work(data0).unwrap();
    }
    println!("filtering vfo1 output");
    for filter in filterlow1.iter_mut() {
        data1 = filter.work(data1).unwrap();
    }
    //filefft(&mut dft, "vfo0", data0.clone(), sps as f64);
    //filefft(&mut dft, "vfo1", data1.clone(), sps as f64);
    // FM decode each stream.
    println!("fm demodulating vfo0 output");
    auds0.write(fm0.work(data0).unwrap());
    println!("fm demodulating vfo1 output");
    auds1.write(fm1.work(data1).unwrap());
}






