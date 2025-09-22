use midi_control::Channel;
use strum::EnumString;
use tracing::*;
use crate::{SynthId, N_STEPS, less_then::UsizeLessThan};

pub type MidiNote = u8;

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub steps: Vec<Step>,
    pub dev: SynthId,
    pub chan: Channel,
    pub name: String,
    pub uuid: usize,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            name: "UNNAMED-1".into(),
            steps: (0..N_STEPS).map(|_| Step::default()).collect(),
            dev: "Default".into(),
            // dev: SynthId::default(),
            chan: Channel::Ch1,
            uuid: 0,
        }
    }
}

impl Track {
    pub fn new(name: Option<String>, uuid: usize, dev: SynthId) -> Self {
        let name = name.unwrap_or(format!("UNNAMED-{uuid}")); 

        Self {
            name,
            steps: (0..N_STEPS).map(|_| Step::default()).collect(),
            dev,
            chan: Channel::Ch1,
            uuid,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, PartialOrd)]
pub struct Step
{
    pub note: Option<MidiNote>,
    pub velocity: Option<u8>,
    pub cmds: (TrackerCmd, TrackerCmd),
}

#[derive(Clone, Copy, Default, Debug, PartialEq, PartialOrd, Eq, Hash)]
pub enum Intervals {
    #[default]
    Root,
    MajThird,
    MinThird,
    FlatFifth,
    Fifth,
    SharpFifth,
    FlatSeventh,
    Seventh,
    SharpSeventh,
}

#[derive(Clone, Default, Debug, PartialEq, PartialOrd, EnumString, strum_macros::Display)]
pub enum RepeatConf {
    #[default]
    HalfStep,
    Step,
}

#[derive(Clone, Default, Debug, PartialEq, PartialOrd, EnumString, strum_macros::Display)]
pub enum TrackerCmd {
    #[default]
    #[strum(to_string = "----")]
    None,
    #[strum(to_string = "CHRD")]
    Chord {
        // chord: Vec<Intervals>
        /// the aditional intervals to play (in semi-tones relative to the root)
        chord: Vec<i8>,
    },
    /// repeates every half step
    #[strum(to_string = "ROLL")]
    Roll {
        /// how many extra times to "roll" what ever is being played. a value of 1 would produce
        /// two 32th notes.
        times: UsizeLessThan<{ N_STEPS * 2 - 1 }>,
        // times: UsizeLessThan<5>,
    },
    /// Repeat every step.
    #[strum(to_string = "RPET")]
    Repeat {
        /// how many times to "repeat" what ever is being played.
        times: UsizeLessThan<{ N_STEPS - 1 }>,
    },
    // // NOTE: maybe remove Swing
    // #[strum(to_string = "SWNG")]
    // Swing {
    //     /// the amount of swing to put on the note
    //     amt: UsizeLessThan<128>,
    // },
    #[strum(to_string = "HOLD")]
    HoldFor { notes: UsizeLessThan<{ N_STEPS }> },
    /// stop all notes on device
    #[strum(to_string = "STOP")]
    Panic,
    #[strum(to_string = "CC{cc_param:->2X}")]
    MidiCmd {
        cc_param: u8,
        arg: u8,
    },
    #[strum(transparent)]
    Custom(Sf2Cmd),
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, EnumString, strum_macros::Display)]
pub enum Sf2Cmd {
    #[strum(to_string = "Atk-")]
    Atk(usize),
    #[strum(to_string = "Dcy-")]
    Dcy(usize),
    #[strum(to_string = "Dcy2")]
    Dcy2(usize),
    #[strum(to_string = "Sus-")]
    Sus(usize),
    #[strum(to_string = "Rel-")]
    Rel(usize),
    #[strum(to_string = "Vol-")]
    Volume(f32),
}

impl Default for Sf2Cmd {
    fn default() -> Self {
        Self::Volume(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_cmd_display() {
        struct MidiCmd<'a>(TrackerCmd, &'a str);

        for MidiCmd(cmd, should_be) in [
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 0,
                    arg: 0,
                    // arg_2: 0,
                },
                "CC-0",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 10,
                    arg: 0,
                    // arg_1: 0,
                    // arg_2: 0,
                },
                "CC-A",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 15,
                    arg: 0,
                    // arg_1: 0,
                    // arg_2: 0,
                },
                "CC-F",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 16,
                    arg: 0,
                    //     arg_1: 0,
                    //     arg_2: 0,
                },
                "CC10",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 126,
                    arg: 0,
                    // arg_1: 0,
                    // arg_2: 0,
                },
                "CC7E",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 127,
                    arg: 0,
                    // arg_1: 0,
                    // arg_2: 0,
                },
                "CC7F",
            ),
            MidiCmd(
                TrackerCmd::MidiCmd {
                    cc_param: 255,
                    arg: 0,
                    // arg_1: 0,
                    // arg_2: 0,
                },
                "CCFF",
            ),
        ] {
            let cmd = format!("{cmd}");
            assert_eq!(
                cmd, should_be,
                "TrackerCmd formating check failed. Cmd formatted to {cmd:?}, when it should should have formatted to {should_be:?}."
            )
        }
    }
}
