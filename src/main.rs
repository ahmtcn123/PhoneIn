use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream, StreamConfig,
};
use crossbeam_channel::{bounded, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

fn create_input(
    device: &cpal::Device,
    config: StreamConfig,
    tx: crossbeam_channel::Sender<Vec<f32>>,
) -> Result<Stream, Box<dyn std::error::Error>> {
    let input_stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Send a chunk of data to the channel
            if let Err(_) = tx.send(data.to_vec()) {
                eprintln!("Failed to send data");
            }
        },
        |err| eprintln!("An error occurred on the input audio stream: {}", err),
        None,
    )?;

    Ok(input_stream)
}

fn create_output(
    device: &cpal::Device,
    config: StreamConfig,
    rx: Receiver<Vec<f32>>,
) -> Result<Stream, Box<dyn std::error::Error>> {
    let output_stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if let Ok(audio_buffer) = rx.try_recv() {
                // Copy only as much as fits in the output buffer
                let len = data.len().min(audio_buffer.len());
                data[..len].copy_from_slice(&audio_buffer[..len]);
            } else {
                // If no data is available, fill with silence
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
            }
        },
        |err| eprintln!("An error occurred on the output audio stream: {}", err),
        None,
    )?;

    Ok(output_stream)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the default host and input device
    let host = cpal::default_host();

    let input_device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let input_config: StreamConfig = input_device.default_input_config()?.into();

    let output_device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let output_config: StreamConfig = output_device.default_output_config()?.into();

    // Check if sample rates match
    if input_config.sample_rate.0 != output_config.sample_rate.0 {
        return Err("Input and output devices have different sample rates".into());
    }

    let (tx, rx) = bounded::<Vec<f32>>(10); // Channel with buffer size of 10 chunks

    let input_stream = create_input(&input_device, input_config, tx)?;
    let output_stream = create_output(&output_device, output_config, rx)?;

    input_stream.play()?;
    output_stream.play()?;

    // Keep the program running
    loop {
        sleep(Duration::from_millis(1));
    }

    // Ok(())
}
