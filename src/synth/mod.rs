use core::panic;
use std::{ops::DerefMut, sync::{Arc, Mutex, RwLock}};
use stepper_synth_backend::{
    pygame_coms::SynthEngineType, synth_engines::{
        wave_table::WaveTableEngine,
        Synth,
        SynthChannel,
        SynthEngine,
    }, HashMap, SampleGen, CHANNEL_SIZE, SAMPLE_RATE
};
use tinyaudio::{run_output_device, OutputDevice, OutputDeviceParameters};
use crate::SynthId;

#[derive(Debug)]
pub struct TabSynth {
    synths: Arc<RwLock<Vec<(SynthId, SynthChannel)>>>,
    /// maps synth friendly names to index values in self.synths.
    /// I'd store the SynthChannel directly but that crashes the app.
    db: Arc<RwLock<HashMap<SynthId, usize>>>,
}

impl TabSynth {
    pub fn new() -> (Self, OutputDevice) {
        let synths = Arc::new(RwLock::new(vec![("Default".to_string(), SynthChannel::from(SynthEngineType::WaveTable))]));

        let device = {
            let synths = synths.clone();

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
                        let value = synths.write().map(|synth| synth.iter().map(|(_name, instrument)| instrument.get_sample()).sum()).unwrap_or(0.0);

                        for sample in samples {
                            *sample = value;
                        }
                    }
                }
            })
        };

        let mut db = HashMap::default();
        db.insert("Default".to_string(), 0);
        let db = Arc::new(RwLock::new(db));

        match device {
            Ok(device) => (Self { synths, db }, device),
            Err(e) => {
                println!("starting audio playback caused error: {e}");
                panic!("{e}");
            }
        }
    }

    // fn do_rename(&self, from: impl ToString, to: impl ToString) -> Result<(), String> {
    //     let from = from.to_string();
    //     let to = to.to_string();

    //     let synth_id = {
    //         let Some(i) = self.synths.read().iter().position(|(id, _synth)| { from == id.read().to_owned() }) else {
    //             return Err(format!("no synth by the name: \"{from}\", found in database"));
    //         };


    //     };

    //     Ok(())
    // }

    pub fn rename(&self, from: impl ToString, to: impl ToString) {
        let from = from.to_string();
        let to = to.to_string();

        self.synths.read().iter_mut().for_each(|(id, _synth)| { if id.to_owned() == from {
            *id = from; 
        }});


    }

    // #[unsafe(no_mangle)]
    // pub fn play(&mut self, note: u8, velocity: u8) {
    //     println!("playing note {note}");
    //     self.synth.write().unwrap().engine.play(note, velocity);
    // }

    // #[unsafe(no_mangle)]
    // pub fn stop(&mut self, note: u8) {
    //     println!("stopping note {note}");
    //     self.synth.write().unwrap().engine.stop(note);
    // }

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
