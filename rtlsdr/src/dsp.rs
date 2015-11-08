#![allow(dead_code)]

/// At the moment the entire DSP code base is built as blocks, which form 
/// pipelines. These pipelines can be branched and converged, and form a
/// graph. The entire namespace is designed with as much performance as possible
/// while still being safe and ergonomic in spirit of the Rust language.
///
/// This entire module will contain various blocks that can be:
///
/// * experimental
/// * easy to learn from
/// * stable
/// * accurate
/// * low resource consumption
/// * high performance
///
/// I hope that this will make this highly attractive to many individuals from
/// those that just want some code to reference for learning to those that need
/// to develope a high performance solution in resource limited environments.
///
/// Since Rust lends itself well to system level work this DSP toolkit of sorts
/// may be very attractive to embedded system development, hopefully!
pub mod block {
    /// This is what all sources and adapters return. It supports the ability
    /// for the output to be valid or invalid, and also provides the ability
    /// to signal that more input data is needed.
    pub enum BlockOutput<O> {
        /// The amount of data provided is not supported, because it was it was
        /// either too many or too few, and it has been discarded. The string
        /// will hopefully contain a useful diagnostic message.
        ErrorInputSizeInvalid(String),
        /// The data was processed, but more if needed to produce some output.
        NeedMoreInput(usize),
        Ready(Vec<O>),
    }

    impl<O> BlockOutput<O> {
        pub fn unwrap(self) -> Vec<O> {
            match self {
                BlockOutput::Ready(v) => v,
                _ => panic!("There was no data to unwrap from BlockOutput."),
            }
        }
    }

    /// This is used by a source so that you can specify exactly how much
    /// or how little data you desire. It provides support for also requesting
    /// what the source block determines if an optimal size for performance,
    /// and it provides support for finite sources so that you can read all 
    /// data from the source.
    pub enum SourceAmount {
        UntilEnd,
        AtLeast(usize),
        Optimal,
        AtMost(usize),
        Between(usize, usize),
    }

    /// This is a block that produces output for input.
    pub trait Adapter<I, O> {
        fn work(&mut self, input: Vec<I>) -> BlockOutput<O>;
    }

    /// This is a block that produces output with no direct input.
    pub trait Source<O> {
        fn read(&mut self, amount: SourceAmount) -> BlockOutput<O>;
    }

    /// This is a block that consumes input with no direct output.
    pub trait Sink<I> {
        fn write(&mut self, input: Vec<I>);
    }

    pub mod transform;

    /// This module contains blocks that will take input and either break
    /// or collect it into specific sized chunks. This is useful when you
    /// are unable to provide a certain sized chunk of input to a block, 
    /// but the block requires that specific size.
    pub mod chunker {
    }

    /// Provides signal data.
    pub mod source {
        /// RTL-SDR Dongle
        ///
        /// http://www.rtl-sdr.com/
        ///
        /// * USB - provides native USB support using the rtlsdr native library
        ///   - if you do not have this installed then this support will not be
        ///   built
        pub mod rtlsdr {
            pub mod usb;
            pub use self::usb::USB;
        }
    }

    /// This module holds anything related to a FFT, including a DFT.
    pub mod fft {
        pub mod fftw;
    }
    
    /// Signal generator or mixer.
    ///
    /// * shift/rotate frequencies
    /// * modulate signals
    pub mod mixer;

    /// Filter signals.
    pub mod filter {
        pub mod crude;
    }

    /// Demodulate signals.
    pub mod demod {
        pub mod fm;
    }

    /// Various sinks for data.
    pub mod sink;
}