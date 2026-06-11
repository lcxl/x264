use core::mem;
use x264::*;
use {Encoder, Encoding, Error, Result};

mod preset;
mod tune;

pub use self::preset::*;
pub use self::tune::*;

/// Builds a new encoder.
pub struct Setup {
    raw: x264_param_t,
}

impl Setup {
    /// Creates a new builder with the specified preset and tune.
    pub fn preset(preset: Preset, tune: Tune, fast_decode: bool, zero_latency: bool) -> Self {
        let mut raw = unsafe { mem::uninitialized() };

        // Name validity verified at compile-time.
        assert_eq!(0, unsafe {
            x264_param_default_preset(
                &mut raw,
                preset.to_cstr(),
                tune.to_cstr(fast_decode, zero_latency),
            )
        });

        Self { raw }
    }

    /// Makes the first pass faster.
    pub fn fastfirstpass(mut self) -> Self {
        unsafe {
            x264_param_apply_fastfirstpass(&mut self.raw);
        }
        self
    }

    /// The video's framerate, represented as a rational number.
    ///
    /// The value is in frames per second.
    pub fn fps(mut self, num: u32, den: u32) -> Self {
        self.raw.i_fps_num = num;
        self.raw.i_fps_den = den;
        self
    }

    /// The encoder's timebase, used in rate control with timestamps.
    ///
    /// The value is in seconds per tick.
    pub fn timebase(mut self, num: u32, den: u32) -> Self {
        self.raw.i_timebase_num = num;
        self.raw.i_timebase_den = den;
        self
    }

    /// Please file an issue if you know what this does, because I have no idea.
    pub fn annexb(mut self, annexb: bool) -> Self {
        self.raw.b_annexb = if annexb { 1 } else { 0 };
        self
    }

    /// Approximately restricts the bitrate.
    ///
    /// The value is in metric kilobits per second.
    pub fn bitrate(mut self, bitrate: i32) -> Self {
        self.raw.rc.i_bitrate = bitrate;
        self
    }

    /// The lowest profile, with guaranteed compatibility with all decoders.
    pub fn baseline(mut self) -> Self {
        unsafe {
            x264_param_apply_profile(&mut self.raw, b"baseline\0" as *const u8 as *const i8);
        }
        self
    }

    /// A useless middleground between the baseline and high profiles.
    pub fn main(mut self) -> Self {
        unsafe {
            x264_param_apply_profile(&mut self.raw, b"main\0" as *const u8 as *const i8);
        }
        self
    }

    /// The highest profile, which almost all encoders support.
    pub fn high(mut self) -> Self {
        unsafe {
            x264_param_apply_profile(&mut self.raw, b"high\0" as *const u8 as *const i8);
        }
        self
    }

    /// Sets the maximum interval between IDR keyframes (GOP size).
    ///
    /// A value of 0 keeps the default (auto). For low-latency remote desktop use
    /// cases a value around 30-60 ensures fast recovery after packet loss.
    pub fn keyint(mut self, max: u32) -> Self {
        self.raw.i_keyint_max = max as i32;
        self
    }

    /// Enable constant quality (CRF) mode
    /// value range is usually 0-51, 0 is lossless, 23 is default, 18-22 is visually lossless
    pub fn crf(mut self, value: f32) -> Self {
        // 1. Set rate control mode to CRF (in x264.h X264_RC_CRF is usually 1)
        self.raw.rc.i_rc_method = 1;
        // 2. Set CRF value
        self.raw.rc.f_rf_constant = value;
        self
    }

    /// Constrains the peak bitrate with VBV (Video Buffering Verifier).
    ///
    /// `max_kbps` caps the instantaneous bitrate (kbit/s) and
    /// `buffer_kbit` sets the VBV buffer size (kbit). Combine with
    /// `crf()` for constrained-quality encoding: quality-driven in the
    /// steady state, rate-capped under motion.
    ///
    /// VBV must be enabled here, at build time, for
    /// `Encoder::reconfig_vbv` to work later — x264 cannot turn VBV on
    /// through `x264_encoder_reconfig` at runtime, it can only adjust
    /// the values of an already-enabled VBV.
    pub fn vbv(mut self, max_kbps: i32, buffer_kbit: i32) -> Self {
        self.raw.rc.i_vbv_max_bitrate = max_kbps;
        self.raw.rc.i_vbv_buffer_size = buffer_kbit;
        self
    }

    /// Build the encoder.
    pub fn build<C>(mut self, csp: C, width: i32, height: i32) -> Result<Encoder>
    where
        C: Into<Encoding>,
    {
        self.raw.i_csp = csp.into().into_raw();
        self.raw.i_width = width;
        self.raw.i_height = height;

        let raw = unsafe { x264_encoder_open(&mut self.raw) };

        if raw.is_null() {
            Err(Error)
        } else {
            Ok(unsafe { Encoder::from_raw(raw) })
        }
    }
}

impl Default for Setup {
    fn default() -> Self {
        let raw = unsafe {
            let mut raw = mem::uninitialized();
            x264_param_default(&mut raw);
            raw
        };

        Self { raw }
    }
}
