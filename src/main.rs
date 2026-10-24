mod app;
mod cmd_watcher;
mod input;
mod keymap;
mod music;
mod synth;
mod terminal;
mod ui;

use std::error::Error;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use app::{Event, Flow, Piano};
use input::InputReader;
use synth::{Sustain, Synth};
use terminal::{FocusReporting, KittyKeyboard, RawMode};

/// キーを離したことが分からない端末で、1音を鳴らし続ける時間
const FALLBACK_NOTE_DURATION: Duration = Duration::from_millis(500);

fn main() -> Result<(), Box<dyn Error>> {
    let synth = Synth::open()?;

    // ガードは宣言と逆順に Drop されるので、フォーカス通知・kitty protocol を戻してから端末設定を戻す
    let raw = RawMode::enable()?;
    let kitty = KittyKeyboard::enable(&raw, &mut io::stdin().lock(), &mut io::stdout())?;
    let kitty_enabled = kitty.is_some();
    // フォーカス通知と左 Cmd 監視は kitty 有効時のみ。従来モードでは CSI I / O の ESC を
    // 終了キーと区別できず、フォーカスも分からないため
    let _focus = kitty_enabled.then(|| FocusReporting::enable(&mut io::stdout())).transpose()?;

    // 端末入力と左 Cmd 監視をそれぞれ別スレッドで待ち、1つのチャネルにまとめる
    let (tx, rx) = mpsc::channel();
    if kitty_enabled {
        cmd_watcher::spawn(tx.clone(), || Event::LeftCmdTap);
    }
    thread::spawn(move || {
        let mut input = InputReader::new(io::stdin(), kitty_enabled);
        while let Ok(Some(event)) = input.next_event() {
            if tx.send(Event::Input(event)).is_err() {
                return;
            }
        }
        let _ = tx.send(Event::InputClosed);
    });

    let sustain = if kitty_enabled { Sustain::UntilRelease } else { Sustain::Fixed(FALLBACK_NOTE_DURATION) };
    let mut piano = Piano::new(synth, sustain);
    piano.redraw()?;
    for event in rx {
        if let Flow::Quit = piano.handle(event)? {
            break;
        }
    }

    println!();
    Ok(())
}
