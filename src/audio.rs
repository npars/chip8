use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(test)]
use mockall::{automock, predicate::*};
use std::error::Error;

#[cfg_attr(test, automock)]
pub trait Audio {
    fn play(&mut self);
    fn pause(&mut self);
}

pub struct Chip8Audio {
    stream: cpal::Stream,
    is_paused: bool,
}

impl Chip8Audio {
    pub fn new() -> Result<Chip8Audio, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device detected");
        let config = device.default_output_config()?;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build_stream::<f32>(&device, &config.into()),
            cpal::SampleFormat::I16 => Self::build_stream::<i16>(&device, &config.into()),
            cpal::SampleFormat::U16 => Self::build_stream::<u16>(&device, &config.into()),
        }?;
        Ok(Chip8Audio {
            stream,
            is_paused: true,
        })
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: cpal::Sample,
    {
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        // Produce a square wave at half amplitude.
        let scale = 0.5f32;
        let mut sample_clock = 0f32;
        let mut next_value = move || {
            sample_clock = (sample_clock + 1.0) % sample_rate;
            (sample_clock * 587.33 * 2.0 * std::f32::consts::PI / sample_rate)
                .sin()
                .signum()
                * scale
        };

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::write_data(data, channels, &mut next_value)
            },
            err_fn,
        )?;
        stream.pause().expect("failed to pause audio");
        Ok(stream)
    }

    fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
    where
        T: cpal::Sample,
    {
        for frame in output.chunks_mut(channels) {
            let value: T = cpal::Sample::from::<f32>(&next_sample());
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

impl Audio for Chip8Audio {
    fn play(&mut self) {
        if self.is_paused {
            self.stream.play().expect("failed to play audio");
            self.is_paused = false;
        }
    }

    fn pause(&mut self) {
        if !self.is_paused {
            self.stream.pause().expect("failed to pause audio");
            self.is_paused = true;
        }
    }
}
