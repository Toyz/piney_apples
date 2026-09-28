//! Playing the engine through the default output device with cpal. The
//! callback locks the engine, renders 48 kHz stereo and resamples it
//! linearly to the device's rate and channel count. With no device (CI, no
//! sound server) there is no stream and nothing fails: the game goes on in
//! silence.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};

use crate::Engine;
use crate::spu::RATE;

pub struct Output {
    _stream: cpal::Stream,
    pub rate: u32,
    pub channels: u16,
}

/// Linear resampling of the engine's 48 kHz stereo to `rate`.
struct Resampler {
    rate: u32,
    /// Position between `prev` and `next` in 1/rate steps of 48 kHz time.
    phase: u64,
    prev: (i16, i16),
    next: (i16, i16),
    scratch: Vec<i16>,
}

impl Resampler {
    fn fill(&mut self, engine: &Mutex<Engine>, frames: usize, mut put: impl FnMut(usize, f32, f32)) {
        let Ok(mut eng) = engine.lock() else { return };
        if self.rate == RATE {
            self.scratch.resize(2 * frames, 0);
            eng.render(&mut self.scratch);
            for i in 0..frames {
                put(i, self.scratch[2 * i] as f32 / 32768.0, self.scratch[2 * i + 1] as f32 / 32768.0);
            }
            return;
        }
        let mut one = [0i16; 2];
        for i in 0..frames {
            // phase counts in units of 1/(rate) of an input sample.
            while self.phase >= self.rate as u64 {
                self.phase -= self.rate as u64;
                self.prev = self.next;
                eng.render(&mut one);
                self.next = (one[0], one[1]);
            }
            let t = self.phase as f32 / self.rate as f32;
            let l = self.prev.0 as f32 + (self.next.0 as f32 - self.prev.0 as f32) * t;
            let r = self.prev.1 as f32 + (self.next.1 as f32 - self.prev.1 as f32) * t;
            put(i, l / 32768.0, r / 32768.0);
            self.phase += RATE as u64;
        }
    }
}

fn build<T>(device: &cpal::Device, config: StreamConfig, engine: Arc<Mutex<Engine>>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut rs = Resampler { rate: config.sample_rate, phase: 0, prev: (0, 0), next: (0, 0), scratch: Vec::new() };
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels.max(1);
                data.fill(T::from_sample(0.0f32));
                rs.fill(&engine, frames, |i, l, r| {
                    let frame = &mut data[i * channels..(i + 1) * channels];
                    match channels {
                        1 => frame[0] = T::from_sample((l + r) * 0.5),
                        _ => {
                            frame[0] = T::from_sample(l);
                            frame[1] = T::from_sample(r);
                        }
                    }
                });
            },
            |e| eprintln!("piney-audio: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

impl Output {
    /// The default device, preferring 48 kHz when it offers it. `Err` when
    /// there is no device or it will not open; the caller carries on silent.
    pub fn open(engine: Arc<Mutex<Engine>>) -> Result<Output, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no output device")?;
        let default = device.default_output_config().map_err(|e| e.to_string())?;
        let mut chosen = default;
        if default.sample_rate() != RATE
            && let Ok(configs) = device.supported_output_configs()
        {
            for c in configs {
                if c.channels() >= 2
                    && c.sample_format() == default.sample_format()
                    && c.min_sample_rate() <= RATE
                    && RATE <= c.max_sample_rate()
                {
                    chosen = c.with_sample_rate(RATE);
                    break;
                }
            }
        }
        let format = chosen.sample_format();
        let config: StreamConfig = chosen.into();
        let (rate, channels) = (config.sample_rate, config.channels);
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, engine),
            SampleFormat::I16 => build::<i16>(&device, config, engine),
            SampleFormat::I32 => build::<i32>(&device, config, engine),
            SampleFormat::U16 => build::<u16>(&device, config, engine),
            SampleFormat::F64 => build::<f64>(&device, config, engine),
            other => Err(format!("unsupported sample format {other}")),
        }?;
        Ok(Output { _stream: stream, rate, channels })
    }
}
