//! 画面表示: ヘルプ・現在の調・弾いた音の履歴を描く

use std::io::{self, Write};

use crate::keymap::{self, CHORD_ROWS};
use crate::music::{self, MUSICAL_KEYS, MusicalKey};
use crate::synth::Sustain;

/// 画面をクリアして全体を描き直す。
/// 履歴が折り返していても崩れないよう、部分的に消さず毎回全体を描く
pub fn redraw(key: &MusicalKey, sustain: Sustain, history: &[String]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[2J\x1b[H")?;
    writeln!(out, "keyboard piano")?;

    // 左手は「キーの行」と「和音名の行」の2行で1段。右手は各段のキー行に並べる
    writeln!(out, "  左手: 伴奏の和音             右手: メロディ")?;
    for (chord_row, (melody_keys, start)) in CHORD_ROWS.iter().zip(keymap::melody_rows()) {
        let chord_keys: String = chord_row.iter().map(|(k, _)| format!("{:<8}", *k as char)).collect();
        let chord_names: String = chord_row.iter().map(|(_, c)| format!("{:<8}", c.name(key))).collect();
        let melody_keys: Vec<_> = melody_keys.iter().map(|&k| (k as char).to_string()).collect();
        let melody_names: Vec<_> = (start..start + melody_keys.len()).map(music::solfege).collect();
        writeln!(out, "    {chord_keys}  {} -> {}", melody_keys.join(" "), melody_names.join(" "))?;
        writeln!(out, "    {chord_names}")?;
    }
    writeln!(out)?;

    let keys: Vec<_> = MUSICAL_KEYS.iter().enumerate().map(|(i, k)| format!("{}:{}", i + 1, k.name)).collect();
    writeln!(out, "  1〜9               -> 調の切り替え ({})", keys.join(" "))?;
    writeln!(out, "  Ctrl+L / Ctrl+H    -> 表示を全部消す / 1つ前を消す")?;
    writeln!(out, "  ESC / Ctrl+C       -> 終了")?;
    match sustain {
        Sustain::UntilRelease => writeln!(out, "  (キーを押している間だけ鳴ります)")?,
        Sustain::Fixed(d) => writeln!(
            out,
            "  (この端末はキーの離上を通知しないため、1音 {}ms 鳴らします)",
            d.as_millis()
        )?,
    }
    writeln!(out)?;

    writeln!(out, "[key {}]", key.name)?;
    for name in history {
        write!(out, "{name} ")?;
    }
    out.flush()
}

/// 履歴の末尾に1つ書き足す (全体を描き直さない)
pub fn append(name: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "{name} ")?;
    out.flush()
}
