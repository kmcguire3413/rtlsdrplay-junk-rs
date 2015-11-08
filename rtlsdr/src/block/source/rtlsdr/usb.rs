#![allow(dead_code)]
#![allow(non_camel_case_types)]
use std::sync::{Mutex, Arc};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::thread;
use std::mem::{transmute, transmute_copy, zeroed, swap, forget};
use dsp::block::{BlockOutput, Source, SourceAmount};
use num::Complex;

mod ffi {
    use libc;

    pub type RTLSDR_DEV = *const ();
    pub type rtlsdr_read_async_cb = extern fn(buf: *mut u8, len: u32, ctx: *const ());

    #[link(name = "rtlsdr")]
    extern "C" {
        pub fn rtlsdr_get_sample_rate(dev: RTLSDR_DEV) -> libc::c_uint;
        pub fn rtlsdr_set_agc_mode(dev: RTLSDR_DEV, on: libc::c_int) -> libc::c_int;
        pub fn rtlsdr_read_sync(dev: RTLSDR_DEV, buf: *mut libc::c_void, max: libc::c_int, nread: *mut libc::c_int) -> libc::c_int;
        pub fn rtlsdr_close(dev: RTLSDR_DEV) -> libc::c_int;
        pub fn rtlsdr_cancel_async(dev: RTLSDR_DEV) -> libc::c_int;
        pub fn rtlsdr_set_offset_tuning(dev: RTLSDR_DEV, on: libc::c_int) -> libc::c_int;
        pub fn rtlsdr_set_sample_rate(dev: RTLSDR_DEV, rate: u32) -> libc::c_int;
        pub fn rtlsdr_reset_buffer(dev: RTLSDR_DEV) -> libc::c_int;
        pub fn rtlsdr_set_freq_correction(dev: RTLSDR_DEV, ppm: libc::c_int) -> libc::c_int;
        pub fn rtlsdr_set_tuner_gain_mode(dev: RTLSDR_DEV, manual: libc::c_int) -> libc::c_int;
        pub fn rtlsdr_set_direct_sampling(dev: RTLSDR_DEV, on: libc::c_int) -> libc::c_int;
        pub fn rtlsdr_set_center_freq(dev: RTLSDR_DEV, freq: u32) -> libc::c_int;
        pub fn rtlsdr_open(outp: *const RTLSDR_DEV, index: u32) -> libc::c_int;
        pub fn rtlsdr_read_async(dev: RTLSDR_DEV, cb: rtlsdr_read_async_cb, ctx: *const (), bufnum: u32, buflen: u32) -> libc::c_int;
    }
}

pub type SAMPLE = u8;

struct RTLSDRInner {
    index:          u32,
    dev:            ffi::RTLSDR_DEV,
    /// This is offloaded from the receiver. It sits
    /// here waiting to be moved to a more permanent
    /// place.
    offload:        Mutex<Vec<Vec<SAMPLE>>>,
    /// Current offload sample count.
    offloadmaxcnt:  usize,
    /// The maximum size of `offload` in samples.
    offloadcurcnt:  AtomicUsize,
    /// Basic communication with the offload thread.
    offloadcomm:    AtomicUsize,
}

#[derive(Clone)]
pub struct USB {
    inner:          Arc<RTLSDRInner>,
}

impl Drop for USB {
    fn drop(&mut self) {

        println!("[rtl-sdr] dev:{:p} waiting for offloader to exit", self.inner.dev);
        while self.inner.offloadcomm.load(Ordering::SeqCst) == 0 {
            unsafe { ffi::rtlsdr_cancel_async(self.inner.dev); }
        }

        unsafe {
            ffi::rtlsdr_close(self.inner.dev);
        }
    }
}

/// A hopefully fast handler for moving the data to a thread safe location
/// that can be accessed by other parts of the program.
extern "C" fn readercb(buf: *mut u8, len: u32, __ctx: *const ()) {
    let ctx: &Arc<RTLSDRInner> = unsafe { transmute(&__ctx) };
    //println!("[offload-thread] handling {} bytes with context {:p}", len, ctx);
    let mut offload = ctx.offload.lock().unwrap();
    if ctx.offloadcurcnt.fetch_add(len as usize, Ordering::SeqCst) + len as usize > ctx.offloadmaxcnt {
        // We have too large of a backlog. This prevents run away memory usage.
        println!("[offload-thread] dropped {} bytes", len);
        ctx.offloadcurcnt.fetch_sub(len as usize, Ordering::SeqCst);
        return;
    }
    let v = unsafe { Vec::from_raw_parts(buf, len as usize, len as usize) };
    offload.push(v);
}

impl Source<Complex<i8>> for USB {
    fn read(&mut self, amount: SourceAmount) -> BlockOutput<Complex<i8>> {
        let _chunk;

        match amount {
            SourceAmount::UntilEnd => panic!("reading -1 count from the RTLSDR USB in async mode will never terminate"),
            SourceAmount::AtLeast(_) => unimplemented!(),
            SourceAmount::AtMost(cnt) => {
                if self.isasync() {
                    println!("asyncread for {}", cnt);
                    _chunk = self.asyncread(cnt * 2);
                } else {
                    println!("syncread for {}", cnt);
                    _chunk = self.syncread((cnt * 2) as isize);
                }
            },
            SourceAmount::Between(_, _) => unimplemented!(),
            SourceAmount::Optimal => {
                if self.isasync() {
                    _chunk = self.asyncreadchunk();
                } else {
                    _chunk = self.syncread(2 * 4096);
                }
            }
        }

        let mut chunk: Vec<Complex<i8>> = unsafe { 
            Vec::from_raw_parts(transmute(_chunk.as_ptr()), _chunk.len() / 2, _chunk.capacity() / 2)
        };
        unsafe { forget(_chunk) };
        // The samples are not yet correct signed bytes, so we correct that here.
        for sample in chunk.iter_mut() {
            sample.re = (sample.re as u8).wrapping_sub(128) as i8;
            sample.im = (sample.im as u8).wrapping_sub(128) as i8;
        }
        BlockOutput::Ready(chunk)        
    }
}

impl USB {
    /// Read a single chunk of unknown size, _ONLY_ if `async` was request when
    /// by the `RTLSDR::new` being called with `async` set to `true`. If not you
    /// must either manual setup async reading or call the syncread method.
    ///
    /// This is going to likely be what the offloader thread read from a single 
    /// USB event. It is generally going to be controlled from somewhere else.
    ///
    /// This provides an efficient way to process the offload queue.
    pub fn asyncreadchunk(&self) -> Vec<SAMPLE> {
        self.inner.offload.lock().unwrap().remove(0)
    }

    /// Read a specified maximum number of samples, _ONLY_ if `async` was 
    /// requested by the `RTLSDR::new` being called with `async` set to `true`.
    /// If not you must either manual setup async reading or call the syncread
    /// method.
    ///
    /// This may be a little inefficient, since it might have to chop up some
    /// buffers to pull out this exact maximum number of bytes if avaliable. If
    /// you need to be efficient then consider using `asyncreadchunk`.
    pub fn asyncread(&self, maxcnt: usize) -> Vec<SAMPLE> {
        let mut offload = self.inner.offload.lock().unwrap();
        let mut out: Vec<u8> = Vec::new();

        while offload.len() > 0 {
            if offload[0].len() + out.len() > maxcnt {
                // Only take part of it, and then exit.
                let togo = maxcnt - out.len();
                let mut n = offload[0].split_off(togo);
                swap(&mut n, &mut offload[0]);
                out.append(&mut n);
                break;
            } else {
                // Take it all.
                out.append(&mut offload.remove(0));
            }
        }

        out
    }

    /// Read a specified maximum number of samples. _WARNING:_ If used with
    /// `async` mode then the behavior is unknown currently.
    ///
    /// TODO: Make function to read _into_ buffer, instead of creating a new
    ///       buffer for each invocation.
    pub fn syncread(&self, maxcnt: isize) -> Vec<SAMPLE> {
        // Create a buffer to place samples we read into.
        let mut buf: Vec<SAMPLE> = Vec::with_capacity(maxcnt as usize);
        let read: usize = 0;
        unsafe {
            ffi::rtlsdr_read_sync(
                self.inner.dev, 
                transmute(buf.as_slice().as_ptr()),
                maxcnt as i32,
                transmute(&read)
            );
            buf.set_len(read);
        }  

        buf
    }

    /// Set the center frequency of the device.
    pub fn setcenterfreq(&self, freq: u32) {
        unsafe {
            ffi::rtlsdr_set_center_freq(self.inner.dev, freq);
        }
    }

    /// Get the current sample rate of the device.
    pub fn getsamplerate(&self) -> u32 {
        unsafe {
            ffi::rtlsdr_get_sample_rate(self.inner.dev)
        }
    }

    /// Sets the sampling frequency.
    pub fn  setsamplerate(&self, freq: u32) {
        unsafe {
            ffi::rtlsdr_set_sample_rate(self.inner.dev, freq);
        }
    }

    /// Will enable or disable manual tuning.
    pub fn setmanualtuner(&self, on: bool) {
        unsafe {
            if on {
                ffi::rtlsdr_set_tuner_gain_mode(self.inner.dev, 1);
            } else {
                ffi::rtlsdr_set_tuner_gain_mode(self.inner.dev, 0);
            }
        }
    }

    /// Create a new controling instance of the device.
    pub fn new(index: u32, asyncmemsize: usize) -> Option<USB> {
        let mut dev: ffi::RTLSDR_DEV;

        unsafe {
            dev = zeroed();
            ffi::rtlsdr_open(&mut dev, index);
        }

        if dev.is_null() {
            Option::None
        } else {
            let instance = USB {
                inner: Arc::new(RTLSDRInner {
                    index:          index,
                    dev:            dev,
                    offload:        Mutex::new(Vec::new()),
                    offloadcurcnt:  AtomicUsize::new(0),
                    offloadmaxcnt:  asyncmemsize,
                    offloadcomm:    AtomicUsize::new(0),
                }),
            };

            unsafe {
                // Just setup some default stuff, until changed.
                ffi::rtlsdr_reset_buffer(dev);
                ffi::rtlsdr_set_center_freq(dev, 99710000);
                //ffi::rtlsdr_set_center_freq(dev, 98900000);
                ffi::rtlsdr_set_direct_sampling(dev, 0);
                ffi::rtlsdr_set_tuner_gain_mode(dev, 0);
                ffi::rtlsdr_set_freq_correction(dev, 0);                
                //ffi::rtlsdr_set_sample_rate(dev, 1008000);
                //ffi::rtlsdr_set_sample_rate(dev, 2048000);
                ffi::rtlsdr_set_sample_rate(dev, 3200000);
                ffi::rtlsdr_set_offset_tuning(dev, 0);
                ffi::rtlsdr_set_agc_mode(dev, 1);
            }

            // If async is support requested, then configure the reader.
            if asyncmemsize > 0 {
                unsafe { 
                    let hack1: usize = transmute(dev);
                    let hack2: usize = transmute_copy(&instance.inner);
                    thread::spawn(move || {
                        let inner: &Arc<RTLSDRInner> = transmute(&hack2);
                        ffi::rtlsdr_read_async(transmute(hack1), readercb, transmute(hack2), 0, 0);
                        inner.offloadcomm.store(1, Ordering::Relaxed);
                    });
                };
            } else {
                instance.inner.offloadcomm.store(1, Ordering::Relaxed);
            }

            Option::Some(instance)
        }
    }

    pub fn isasync(&self) -> bool {
        if self.inner.offloadcomm.load(Ordering::Relaxed) == 0 {
            true
        } else {
            false
        }
    }
}