use core::panic;
use std::sync::{Arc, Mutex, RwLock};
use stepper_synth_backend::{
    pygame_coms::SynthEngineType,
    synth_engines::{
        wave_table::WaveTableEngine,
        Synth,
        SynthChannel,
        SynthEngine,
    },
    SampleGen,
    CHANNEL_SIZE,
    SAMPLE_RATE,
};
use tinyaudio::{run_output_device, OutputDevice, OutputDeviceParameters};

#[derive(Debug)]
pub struct TabSynth {
    pub synth: Arc<RwLock<SynthChannel>>,
}

impl TabSynth {
    pub fn new() -> (Self, OutputDevice) {
        let synth = Arc::new(RwLock::new(SynthChannel::from(SynthEngineType::WaveTable)));

        let device = {
            let synth = synth.clone();

            // move || {
            let params = OutputDeviceParameters {
                channels_count: 1,
                sample_rate: SAMPLE_RATE as usize,
                // channel_sample_count: 2048,
                channel_sample_count: CHANNEL_SIZE,
            };
            // NOTE: must stay in this thread so that it stays in scope
            run_output_device(params, {

                move |data| {
                    for samples in data.chunks_mut(params.channels_count) {
                        let value = synth
                            .write()
                            .map(|mut synth| synth.get_sample())
                            .unwrap_or(0.0);

                        for sample in samples {
                            *sample = value;
                        }
                    }
                }
            })
        };

        match device {
            Ok(device) => (Self { synth }, device),
            Err(e) => {
                println!("starting audio playback caused error: {e}");
                panic!("{e}");
            }
        }
    }

    #[unsafe(no_mangle)]
    pub fn play(&mut self, note: u8, velocity: u8) {
        println!("playing note {note}");
        self.synth.write().unwrap().engine.play(note, velocity);
    }

    #[unsafe(no_mangle)]
    pub fn stop(&mut self, note: u8) {
        println!("stopping note {note}");
        self.synth.write().unwrap().engine.stop(note);
    }

    // #[unsafe(no_mangle)]
    // pub fn bend(&mut self, bend: i16) {
    //     println!("bending pitch by {bend} / 16_383");
    //     // self.synth.lock().unwrap().get_engine().stop(note);
    //     self.synth.lock().unwrap().engine.bend(bend);
    // }
    //
    // #[unsafe(no_mangle)]
    // pub fn unbend(&mut self) {
    //     println!("unbending bending pitch");
    //     // self.synth.lock().unwrap().get_engine().stop(note);
    //     self.synth.lock().unwrap().engine.unbend();
    // }
}

// #[unsafe(no_mangle)]
pub fn make_synth() -> (TabSynth, OutputDevice) {
    let (mut synth, dev) = TabSynth::new();

    (synth, dev)
}
