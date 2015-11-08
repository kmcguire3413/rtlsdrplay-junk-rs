use dsp::block::{Sink, BlockOutput};
use num::Complex;
use std::mem::transmute;

pub struct WaveFileI8 {
    path:           String,
    sps:            u32,
    buf:            Vec<Vec<i8>>,
}

impl WaveFileI8 {
    pub fn new(path: String, sps: u32) -> WaveFileI8 {
        WaveFileI8 {
            path:   path,
            sps:    sps,
            buf:    Vec::new(),
        }
    }
}

#[inline]
fn u16tou8ale(v: u16) -> [u8; 2] {
    [
        v as u8,
        (v >> 8) as u8,
    ]
}

// little endian
#[inline]
fn u32tou8ale(v: u32) -> [u8; 4] {
    [
        v as u8,
        (v >> 8) as u8,
        (v >> 16) as u8,
        (v >> 24) as u8,
    ]
}

// big endian
#[inline]
fn u32tou8abe(v: u32) -> [u8; 4] {
    [
        (v >> 24) as u8,
        (v >> 16) as u8,
        (v >> 8) as u8,
        v as u8,
    ]
}

impl Drop for WaveFileI8 {
    fn drop(&mut self) {
        use std::fs::File;
        use std::io::Write;
        let mut fd = File::create(self.path.clone()).unwrap();
        let mut datatotalsize: u32 = 0;
        for buf in self.buf.iter() {
            datatotalsize += buf.len() as u32;
        }

        fd.write("RIFF".as_bytes());                                                    // 4
        fd.write(&u32tou8ale((datatotalsize + 44) - 8)); // filesize - 8                // 4
        fd.write("WAVE".as_bytes());       //                                           // 4
        fd.write("fmt ".as_bytes());       // <format marker>                           // 4
        fd.write(&u32tou8ale(16));         // <format data length>                      // 4
        fd.write(&u16tou8ale(1));          // PCM                                       // 2
        fd.write(&u16tou8ale(1));          // 1 channel                                 // 2
        fd.write(&u32tou8ale(self.sps));   // sample frequency/rate                     // 4
        fd.write(&u32tou8ale(self.sps));   // sps * bitsize * channels / 8 (byte rate)  // 4
        fd.write(&u16tou8ale(1));          // bitsize * channels / 8  (block-align)     // 2
        fd.write(&u16tou8ale(8));          // bits per sample                           // 2
        fd.write("data".as_bytes());       // <data marker>                             // 4
        fd.write(&u32tou8ale(datatotalsize));  // datasize = filesize - 44              // 4
        for buf in self.buf.iter() {
            let vu8: &[u8] = unsafe { transmute(buf.as_slice()) };
            fd.write(vu8);
        }
    }
}

impl Sink<i8> for WaveFileI8 {
    fn write(&mut self, input: Vec<i8>) {
        self.buf.push(input);
    }
}
