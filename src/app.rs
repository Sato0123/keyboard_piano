//! アプリ本体: キーイベントを受けて、音を鳴らす・止める・画面を更新する

use std::collections::HashMap;
use std::io;

use crate::input::{Key, KeyEvent, KeyEventKind};
use crate::keymap::{self, Note};
use crate::music::{self, MUSICAL_KEYS, MusicalKey};
use crate::synth::{HeldNote, Sustain, Synth};
use crate::ui;

/// 1音あたりの音量。伴奏の和音とメロディが重なってもクリップしにくい値にしている。
/// 伴奏はメロディが埋もれないよう少し小さめ
const MELODY_AMPLITUDE: f32 = 0.12;
const CHORD_AMPLITUDE: f32 = 0.07;

/// キーイベントを解釈した結果、アプリがやること
#[derive(Debug, PartialEq)]
enum Action {
    Quit,
    ClearHistory,
    UndoHistory,
    SetMusicalKey(usize),
    Press(u8),
    Release(u8),
    Ignore,
}

fn action_for(event: KeyEvent) -> Action {
    use KeyEventKind::*;
    match (event.kind, event.key, event.ctrl) {
        (Press, Key::Escape, _) | (Press, Key::Char(b'c'), true) => Action::Quit,
        (Press, Key::Char(b'l'), true) => Action::ClearHistory,
        (Press, Key::Char(b'h'), true) | (Press, Key::Backspace, _) => Action::UndoHistory,
        (Press, Key::Char(c @ b'1'..=b'9'), false) => Action::SetMusicalKey((c - b'1') as usize),
        (Press, Key::Char(c), false) => Action::Press(c),
        // Ctrl を押しながら離しても音は止める
        (Release, Key::Char(c), _) => Action::Release(c),
        _ => Action::Ignore,
    }
}

pub enum Flow {
    Continue,
    Quit,
}

pub struct Piano {
    synth: Synth,
    sustain: Sustain,
    /// MUSICAL_KEYS の添字
    musical_key: usize,
    /// 押しっぱなしのキーと、その音 (Sustain::UntilRelease の時だけ使う)
    held: HashMap<u8, HeldNote>,
    /// 画面に表示している弾いた音 (Ctrl+L / Ctrl+H で消す)
    history: Vec<String>,
}

impl Piano {
    pub fn new(synth: Synth, sustain: Sustain) -> Self {
        Self { synth, sustain, musical_key: 0, held: HashMap::new(), history: Vec::new() }
    }

    pub fn redraw(&self) -> io::Result<()> {
        ui::redraw(self.current_key(), self.sustain, &self.history)
    }

    pub fn handle(&mut self, event: KeyEvent) -> io::Result<Flow> {
        match action_for(event) {
            Action::Quit => return Ok(Flow::Quit),
            Action::ClearHistory => {
                self.history.clear();
                self.redraw()?;
            }
            Action::UndoHistory => {
                self.history.pop();
                self.redraw()?;
            }
            Action::SetMusicalKey(index) => {
                self.musical_key = index;
                self.redraw()?;
            }
            Action::Press(key) => self.press(key)?,
            Action::Release(key) => self.release(key),
            Action::Ignore => {}
        }
        Ok(Flow::Continue)
    }

    fn current_key(&self) -> &'static MusicalKey {
        &MUSICAL_KEYS[self.musical_key]
    }

    fn press(&mut self, key: u8) -> io::Result<()> {
        let Some(note) = keymap::note_for(key) else {
            return Ok(());
        };
        if self.held.contains_key(&key) {
            return Ok(());
        }

        let musical_key = self.current_key();
        let (degrees, amplitude, name) = match note {
            Note::Melody(degree) => (vec![degree], MELODY_AMPLITUDE, music::note_name(degree)),
            Note::Chord(chord) => (chord.degrees(), CHORD_AMPLITUDE, format!("[{}]", chord.name(musical_key))),
        };
        let freqs: Vec<_> = degrees.into_iter().map(|d| music::frequency(d, musical_key)).collect();

        let held = self.synth.play(&freqs, amplitude, self.sustain);
        if self.sustain == Sustain::UntilRelease {
            self.held.insert(key, held);
        }

        ui::append(&name)?;
        self.history.push(name);
        Ok(())
    }

    fn release(&mut self, key: u8) {
        if let Some(held) = self.held.remove(&key) {
            held.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key, ctrl: bool) -> KeyEvent {
        KeyEvent { key, ctrl, kind: KeyEventKind::Press }
    }

    #[test]
    fn maps_key_events_to_actions() {
        assert_eq!(action_for(press(Key::Escape, false)), Action::Quit);
        assert_eq!(action_for(press(Key::Char(b'c'), true)), Action::Quit);
        assert_eq!(action_for(press(Key::Char(b'c'), false)), Action::Press(b'c'));
        assert_eq!(action_for(press(Key::Char(b'l'), true)), Action::ClearHistory);
        assert_eq!(action_for(press(Key::Char(b'h'), true)), Action::UndoHistory);
        assert_eq!(action_for(press(Key::Backspace, false)), Action::UndoHistory);
        assert_eq!(action_for(press(Key::Char(b'5'), false)), Action::SetMusicalKey(4));
        assert_eq!(
            action_for(KeyEvent { key: Key::Char(b'v'), ctrl: false, kind: KeyEventKind::Release }),
            Action::Release(b'v')
        );
        assert_eq!(
            action_for(KeyEvent { key: Key::Char(b'v'), ctrl: false, kind: KeyEventKind::Repeat }),
            Action::Ignore
        );
    }
}
