//! アプリ本体: キーイベントを受けて、音を鳴らす・止める・画面を更新する

use std::collections::HashMap;
use std::io;

use crate::input::{InputEvent, Key, KeyEvent, KeyEventKind};
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
    /// 左 Cmd / Tab: 和音ボタンのセットを切り替える
    NextChordSet,
    /// スペース: 押している間に弾いたメロディを半音上げる
    SharpOn,
    SharpOff,
    Press(u8),
    Release(u8),
    Ignore,
}

fn action_for(event: KeyEvent) -> Action {
    use KeyEventKind::*;
    // Ctrl / Cmd を押しながらの文字キーはショートカット扱いで、音は鳴らさない
    let plain = !event.ctrl && !event.cmd;
    match (event.kind, event.key, event.ctrl) {
        (Press, Key::Escape, _) | (Press, Key::Char(b'c'), true) => Action::Quit,
        (Press, Key::Char(b'l'), true) => Action::ClearHistory,
        (Press, Key::Char(b'h'), true) | (Press, Key::Backspace, _) => Action::UndoHistory,
        (Press, Key::LeftCmd | Key::Tab, _) => Action::NextChordSet,
        (Press, Key::Char(c @ b'1'..=b'9'), _) if plain => Action::SetMusicalKey((c - b'1') as usize),
        (Press, Key::Char(b' '), _) => Action::SharpOn,
        (Release, Key::Char(b' '), _) => Action::SharpOff,
        (Press, Key::Char(c), _) if plain => Action::Press(c),
        // 修飾キーを押しながら離しても音は止める
        (Release, Key::Char(c), _) => Action::Release(c),
        _ => Action::Ignore,
    }
}

/// アプリが受け取るイベント (端末入力と左 Cmd 監視の両方から届く)
pub enum Event {
    Input(InputEvent),
    /// 端末入力が閉じた / 読めなくなった
    InputClosed,
    LeftCmdTap,
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
    /// CHORD_SETS の添字
    chord_set: usize,
    /// 次に弾くメロディを半音上げるか。Sustain::Fixed (離上が分からない端末) では
    /// スペースを押しっぱなしにできないので、1音使ったら戻す
    sharp: bool,
    /// 端末ウィンドウが前面にあるか。左 Cmd は端末外の押下も拾うので、前面の時だけ反応する
    focused: bool,
    /// 押しっぱなしのキーと、その音 (Sustain::UntilRelease の時だけ使う)
    held: HashMap<u8, HeldNote>,
    /// 画面に表示している弾いた音 (Ctrl+L / Ctrl+H で消す)
    history: Vec<String>,
}

impl Piano {
    pub fn new(synth: Synth, sustain: Sustain) -> Self {
        Self { synth, sustain, musical_key: 0, chord_set: 0, sharp: false, focused: true, held: HashMap::new(), history: Vec::new() }
    }

    pub fn redraw(&self) -> io::Result<()> {
        ui::redraw(self.current_key(), self.chord_set, self.sustain, &self.history)
    }

    pub fn handle(&mut self, event: Event) -> io::Result<Flow> {
        match event {
            Event::Input(InputEvent::Key(key)) => return self.handle_key(key),
            Event::Input(InputEvent::Focus(focused)) => self.focused = focused,
            Event::InputClosed => return Ok(Flow::Quit),
            Event::LeftCmdTap if self.focused => self.next_chord_set()?,
            Event::LeftCmdTap => {}
        }
        Ok(Flow::Continue)
    }

    fn handle_key(&mut self, event: KeyEvent) -> io::Result<Flow> {
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
            Action::NextChordSet => self.next_chord_set()?,
            Action::SharpOn => self.sharp = true,
            Action::SharpOff => self.sharp = false,
            Action::Press(key) => self.press(key)?,
            Action::Release(key) => self.release(key),
            Action::Ignore => {}
        }
        Ok(Flow::Continue)
    }

    fn next_chord_set(&mut self) -> io::Result<()> {
        self.chord_set = (self.chord_set + 1) % keymap::CHORD_SETS.len();
        self.redraw()
    }

    fn current_key(&self) -> &'static MusicalKey {
        &MUSICAL_KEYS[self.musical_key]
    }

    fn press(&mut self, key: u8) -> io::Result<()> {
        let Some(note) = keymap::note_for(key, self.chord_set) else {
            return Ok(());
        };
        if self.held.contains_key(&key) {
            return Ok(());
        }

        let musical_key = self.current_key();
        let (freqs, amplitude, name) = match note {
            Note::Melody(degree) => {
                let sharp = self.sharp;
                if let Sustain::Fixed(_) = self.sustain {
                    self.sharp = false;
                }
                let freq = music::frequency(degree, sharp, musical_key);
                (vec![freq], MELODY_AMPLITUDE, music::note_name(degree, sharp))
            }
            Note::Chord(chord) => {
                (chord.frequencies(musical_key), CHORD_AMPLITUDE, format!("[{}]", chord.name(musical_key)))
            }
        };

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
        KeyEvent { key, ctrl, cmd: false, kind: KeyEventKind::Press }
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
        assert_eq!(action_for(press(Key::Char(b' '), false)), Action::SharpOn);
        assert_eq!(action_for(press(Key::LeftCmd, false)), Action::NextChordSet);
        assert_eq!(action_for(press(Key::Tab, false)), Action::NextChordSet);
        let cmd_q = KeyEvent { key: Key::Char(b'q'), ctrl: false, cmd: true, kind: KeyEventKind::Press };
        assert_eq!(action_for(cmd_q), Action::Ignore);
        assert_eq!(
            action_for(KeyEvent { key: Key::Char(b' '), ctrl: false, cmd: false, kind: KeyEventKind::Release }),
            Action::SharpOff
        );
        assert_eq!(
            action_for(KeyEvent { key: Key::Char(b'v'), ctrl: false, cmd: false, kind: KeyEventKind::Release }),
            Action::Release(b'v')
        );
        assert_eq!(
            action_for(KeyEvent { key: Key::Char(b'v'), ctrl: false, cmd: false, kind: KeyEventKind::Repeat }),
            Action::Ignore
        );
    }
}
