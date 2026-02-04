use crate::tracks::{Track, TrackerCmd};
use android_usbser::usb;
use crossbeam::channel::{Receiver, Sender, unbounded};
use dioxus::prelude::*;
use lazy_static::lazy_static;
use midi_control::{ControlEvent, KeyEvent, MidiMessage};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    str::FromStr,
    sync::Mutex,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread::{JoinHandle, spawn},
    time::{Duration, SystemTime},
};
use stepper_synth_backend::{
    CHANNEL_SIZE, KnobCtrl, MidiControlled, SAMPLE_RATE, SampleGen,
    synth_engines::{Synth, SynthEngine, SynthModule},
};
use synth::{TabSynth, make_synth};
use tracing::*;

pub mod less_then;
pub mod synth;
pub mod tracks;

pub type SynthId = String;
// pub type InstrumentId = String;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const N_STEPS: usize = 128;

lazy_static! {
    pub static ref CBEAM_CHANNELS: (Sender<MidiMessage>, Receiver<MidiMessage>) = unbounded();
    pub static ref MIDI_SEND: Sender<MidiMessage> = CBEAM_CHANNELS.0.clone();
    pub static ref MIDI_RECV: Receiver<MidiMessage> = CBEAM_CHANNELS.1.clone();
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MiddleColView {
    Section,
    Pattern,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Colums {
    Note,
    Velocity,
    Cmd1,
    Cmd2,
}

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");

    // needed bc audio output will fail if its started too soon.
    // let (synth, output_dev) = make_synth();
    let synth = make_synth();

    // let _jh = spawn(move || {
    //         while let Ok(msg) = MIDI_RECV.recv() {
    //             if let Ok(ref mut synth) = synth.synth.write() {
    //                 match msg {
    //                     MidiMessage::Invalid => {
    //                         // error!("system received an invalid MIDI message.");
    //                     }
    //                     MidiMessage::NoteOn(_, KeyEvent { key, value }) => {
    //                         synth.engine.play(key, value)
    //                     }
    //                     MidiMessage::NoteOff(_, KeyEvent { key, value: _ }) => {
    //                         synth.engine.stop(key)
    //                     }
    //                     MidiMessage::PitchBend(_, lsb, msb) => {
    //                         let bend =
    //                             i16::from_le_bytes([lsb, msb]) as f32 / (32_000.0 * 0.5) - 1.0;

    //                         if bend > 0.02 || bend < -0.020 {
    //                             synth.engine.bend(bend);
    //                         } else {
    //                             synth.engine.unbend();
    //                         }
    //                     }
    //                     MidiMessage::ControlChange(_, ControlEvent { control, value }) => {
    //                         let value = value as f32 / 127.0;
    //                         // let effects = self.target_effects;

    //                         match synth.engine {
    //                             SynthModule::WaveTable(ref mut wt) => {
    //                                 wt.synth.midi_input(&msg);
    //                             }
    //                             ref mut engine => {
    //                                 match control {
    //                                     70 => engine.knob_1(value),
    //                                     71 => engine.knob_2(value),
    //                                     72 => engine.knob_3(value),
    //                                     73 => engine.knob_4(value),
    //                                     74 => engine.knob_5(value),
    //                                     75 => engine.knob_6(value),
    //                                     76 => engine.knob_7(value),
    //                                     77 => engine.knob_8(value),
    //                                     1 => engine.volume_swell(value),
    //                                     _ => {
    //                                         // info!("CC message => {control}-{value}");
    //                                         false
    //                                     }
    //                                 };
    //                             }
    //                         }
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //         }
    //     }
    // );

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let middle_view = use_signal(|| MiddleColView::Section);
    let sections = use_signal(|| {
        vec![
            Track::default(),
            Track::new(Some("Another-Section".into()), 1, "Default".into()),
        ]
    });
    let displaying_uuid = use_signal(|| 0usize);
    // used to give context to the edit note/velcity/cmd-1/cmd-2
    let edit_cell = use_signal(|| None);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        main {
            div {
                id: "left-col",
                LeftCol { middle_view, sections, displaying: displaying_uuid, edit_cell }
            }
            div {
                id: "middle-col",
                MiddleCol { middle_view, sections, displaying: displaying_uuid, edit_cell }

                if edit_cell.read().is_some() && middle_view() == MiddleColView::Section {
                    EditSectionMenu { sections, displaying: displaying_uuid, edit_cell }
                }
            }
            div {
                id: "right-col",
                PlayTone {  }
            }
        }
    }
}

#[component]
fn EditSectionMenu(
    sections: Signal<Vec<Track>>,
    displaying: Signal<usize>,
    edit_cell: Signal<Option<(usize, Colums)>>,
) -> Element {
    let note = use_signal(|| {
        if let Some((row, cell)) = edit_cell() {
            sections.read()[displaying()].steps[row]
                .note
                .unwrap_or(12u8)
        } else {
            0u8
        }
    });
    let velocity = use_signal(|| 85u8);
    let cmd = use_signal(|| TrackerCmd::None);

    rsx! {
        div {
            id: "edit-menu",
            class: "col",

            div {
                id: "set-menu",
                class: "row set-menu",

                div {
                    class: "button",
                    onclick: move |_| {
                        edit_cell.set(None);
                    },

                    "ESC"
                }

                div {
                    class: "button",

                    onclick: move |_| {
                        if let Some((row, cell)) = edit_cell() {
                            info!("{row} => {cell:?}");

                            match cell {
                                Colums::Note => {
                                    // set note
                                    sections.write()[displaying()].steps[row].note = None;
                                }
                                Colums::Velocity => {
                                    // set velocity
                                    sections.write()[displaying()].steps[row].velocity = None;
                                }
                                Colums::Cmd1 => {
                                    // set cmd
                                    sections.write()[displaying()].steps[row].cmds.0 = TrackerCmd::None;
                                }
                                Colums::Cmd2 => {
                                    // set cmd
                                    sections.write()[displaying()].steps[row].cmds.1 = TrackerCmd::None;
                                }
                            }
                        }

                        edit_cell.set(None);
                    },

                    "DEL"
                }

                div {
                    class: "button",

                    onclick: move |_| {
                        if let Some((row, cell)) = edit_cell() {
                            info!("{row} => {cell:?}");

                            match cell {
                                Colums::Note => {
                                    // set note
                                    sections.write()[displaying()].steps[row].note = Some(note());

                                    // set velocity if not yet set
                                    if sections()[displaying()].steps[row].velocity.is_none() {
                                        sections.write()[displaying()].steps[row].velocity = Some(85);
                                    }

                                    info!("set note to {:?}", sections()[displaying()].steps[row].note);
                                }
                                Colums::Velocity => {
                                    // set velocity
                                    sections.write()[displaying()].steps[row].velocity = Some(velocity())
                                }
                                Colums::Cmd1 => {
                                    // set cmd
                                    sections.write()[displaying()].steps[row].cmds.0 = cmd();
                                }
                                Colums::Cmd2 => {
                                    // set cmd
                                    sections.write()[displaying()].steps[row].cmds.1 = cmd();
                                }
                            }
                        }

                        edit_cell.set(None);
                    },

                    "SET"
                }
            }


            if let Some((row, cell)) = edit_cell() {
                match cell {
                    Colums::Note => rsx! { EditNote { note } },
                    _ => { rsx! { } }
                }
            }
        }
    }
}

#[component]
fn EditNote(note: Signal<u8>) -> Element {
    // let original_note = display_midi_note(note());
    let mut octave = use_signal(|| (note() / 12) as i8);
    let mut name = use_signal(|| (note() % 12) as i8);
    let note_names = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];

    rsx! {
        // div {
            // class: "row
            div {
                class: "xx-large super-center",

                "Octave"
            }
            div {
                class: "row space-around",

                div {
                    class: "button large",
                    onclick: move |_| {
                        octave.set(
                            (if octave() > 1 {
                                (octave() - 1)}
                            else {
                                9
                            }) % 10
                        );
                        note.set((name() + octave() * 12) as u8);
                    },
                    "<-"
                }
                div {
                    class: "large",
                    "{octave.read()}"
                }
                div {
                    class: "button large",
                    onclick: move |_| {
                        octave.set((octave() % 9) + 1);
                        note.set((name() + octave() * 12) as u8);
                    },
                    "->"
                }
            }
            div {
                class: "row space-around",

                for (i, display_name) in note_names.iter().enumerate() {
                    div {
                        class: "button large",
                        onclick: move |_| {
                            name.set(i as i8);
                            note.set((name() + octave() * 12) as u8);
                        },
                        "{display_name}"
                    }
                }
            }
            div {
                class: "row space-around",

                div {
                    class: "xx-large",
                    "{display_midi_note(note())}"
                }
            }
        // }
    }
}

#[component]
fn MiddleCol(
    middle_view: Signal<MiddleColView>,
    sections: Signal<Vec<Track>>,
    displaying: Signal<usize>,
    edit_cell: Signal<Option<(usize, Colums)>>,
) -> Element {
    rsx! {
        div {
            id: "middle-main",
            if middle_view() == MiddleColView::Section {
                SectionDisplay { middle_view, sections, displaying, edit_cell }
            } else if middle_view() == MiddleColView::Pattern {}
        }
    }
}

#[component]
fn SectionDisplay(
    middle_view: Signal<MiddleColView>,
    sections: Signal<Vec<Track>>,
    displaying: Signal<usize>,
    edit_cell: Signal<Option<(usize, Colums)>>,
) -> Element {
    rsx! {
        div {
            id: "section-display-header",
            div { "Line" }
            div { "Note" }
            div { "Vel" }
            div { "Cmd1" }
            div { "Cmd2" }
        }

        div {
            id: "section-scroll-list",

            div {
                id: "section-scroll-div",

                for (i, step) in sections()[displaying()].steps.iter().enumerate() {
                    div {
                        class: "section-scroll-item",

                        div {
                            class: "section-row",
                            id: {
                                if i % 2 == 0 {
                                    "row-light"
                                } else {
                                    "row-dark"
                                }
                            },

                            // Line Number
                            div {
                                class: "lin-number",
                                // "{i:->2X}"
                                "{i + 1:->3}"
                            }
                            // Note
                            div {
                                onclick: move |_| {
                                    // open edit menu with context
                                    if edit_cell.read().is_none() {
                                        edit_cell.set(Some((i, Colums::Note)));
                                    }
                                },
                                class: "button super-center",

                                "{step.note.map(display_midi_note).unwrap_or(\"---\".into())}"
                            }
                            // Velocity
                            div {
                                onclick: move |_| {
                                    // open edit menu with context
                                    if edit_cell.read().is_none() {
                                        edit_cell.set(Some((i, Colums::Velocity)));
                                    }
                                },
                                class: "button super-center",

                                if sections()[displaying()].steps[i].note.is_some() {
                                    // "{step.velocity.unwrap_or(85):->3X}"
                                    "{step.velocity.unwrap_or(85):->3}"
                                } else {
                                    "---"
                                }
                            }
                            // CMD 1
                            div {
                                onclick: move |_| {
                                    // open edit menu with context
                                    if edit_cell.read().is_none() {
                                        edit_cell.set(Some((i, Colums::Cmd1)));
                                    }
                                },
                                class: "button super-center",

                                "{step.cmds.0}"
                            }
                            // CMD 2
                            div {
                                onclick: move |_| {
                                    // open edit menu with context
                                    if edit_cell.read().is_none() {
                                        edit_cell.set(Some((i, Colums::Cmd2)));
                                    }
                                },
                                class: "button super-center",

                                "{step.cmds.1}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LeftCol(
    middle_view: Signal<MiddleColView>,
    sections: Signal<Vec<Track>>,
    displaying: Signal<usize>,
    edit_cell: Signal<Option<(usize, Colums)>>,
) -> Element {
    let mut listing = use_signal(|| MiddleColView::Section);
    let view_sections = || listing() == MiddleColView::Section;

    rsx! {
        div {
            id: "section-pattern-select",

            div {
                class: "button col normal-text",
                onclick: move |_| listing.set(MiddleColView::Section),

                div { "Section" }
                div {
                    class: {
                        let mut classes = vec!["led"];
                        if view_sections() { classes.push("led-on") }
                        classes.join(" ")
                    },
                }
            }
            div {
                class: "button col normal-text",
                onclick: move |_| listing.set(MiddleColView::Pattern),

                div { "Pattern" }
                div {
                    class: {
                        let mut classes = vec!["led"];

                        if !view_sections() { classes.push("led-on") }

                        classes.join(" ")
                    },
                }
            }
        }

        div {
            id: "nav-list",

            for (i, (name, uuid)) in match listing() {
                MiddleColView::Section => {
                    sections().iter().map(|section| (section.name.clone(), section.uuid)).enumerate().collect::<Vec<_>>()
                }
                MiddleColView::Pattern => {
                    [].iter().map(|pattern: &(String, usize)| pattern.to_owned()).enumerate().collect::<Vec<_>>()
                }
            } {
                // TODO: add edit-name button here
                div {
                    id: {
                        if (listing() == middle_view()) && (uuid == displaying()) {
                            "displaying-sp".to_string()
                        } else {
                            "".into()
                        }
                    },
                    class: "button nav-item",
                    onclick: move |_| {
                        middle_view.set(listing());
                        displaying.set(uuid);
                        edit_cell.set(None);
                    },
                    "{name}"
                }
                // TODO: add deleat track button here
            }
        }
    }
}

pub fn display_midi_note(midi_note: u8) -> String {
    let note_name_i = midi_note % 12;
    let octave = midi_note / 12;

    let note_names = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    let note_name = note_names[note_name_i as usize];

    format!("{note_name}{octave:X}")
}

#[component]
fn PlayTone() -> Element {
    let mut playing = false;

    rsx! {
        button { onclick: move |_| {
            let send = if !playing {
                MIDI_SEND.send(MidiMessage::NoteOn(midi_control::Channel::Ch1, KeyEvent { key: 48, value: 90 }))
            } else {
                MIDI_SEND.send(MidiMessage::NoteOff(midi_control::Channel::Ch1, KeyEvent { key: 48, value: 90 }))
            };

            if let Err(e) = send {
                error!("{e}");
            } else {
                playing = !playing;
                info!("playing: {playing}");
            }
        }, "Play Example Tone"  }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn note_display() {
        assert_eq!(display_midi_note(60), "C-4");
    }
}

