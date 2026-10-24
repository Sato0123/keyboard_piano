mod app;
mod input;
mod keymap;
mod music;
mod synth;
mod terminal;
mod ui;

use std::error::Error;
use std::io;
use std::time::Duration;

use app::{Flow, Piano};
use input::InputReader;
use synth::{Sustain, Synth};
use terminal::{KittyKeyboard, RawMode};

/// キーを離したことが分からない端末で、1音を鳴らし続ける時間
const FALLBACK_NOTE_DURATION: Duration = Duration::from_millis(500);

fn main() -> Result<(), Box<dyn Error>> {
    let synth = Synth::open()?;

    // ガードは宣言と逆順に Drop されるので、kitty protocol を戻してから端末設定を戻す
    let raw = RawMode::enable()?;
    let mut stdin = io::stdin().lock();
    let kitty = KittyKeyboard::enable(&raw, &mut stdin, &mut io::stdout())?;

    let sustain = if kitty.is_some() { Sustain::UntilRelease } else { Sustain::Fixed(FALLBACK_NOTE_DURATION) };
    let mut piano = Piano::new(synth, sustain);
    let mut input = InputReader::new(stdin, kitty.is_some());

    piano.redraw()?;
    while let Some(event) = input.next_event()? {
        if let Flow::Quit = piano.handle(event)? {
            break;
        }
    }

    println!();
    Ok(())
}
