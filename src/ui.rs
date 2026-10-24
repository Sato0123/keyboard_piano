//! 画面表示: ヘルプ・現在の調・弾いた音の履歴を描く

use std::io::{self, Write};

use crate::keymap::{self, CHORD_KEY_ROWS, CHORD_SETS};
use crate::music::{self, MUSICAL_KEYS, MusicalKey};
use crate::synth::Sustain;

/// 画面をクリアして全体を描き直す。
/// 履歴が折り返していても崩れないよう、部分的に消さず毎回全体を描く
pub fn redraw(key: &MusicalKey, chord_set: usize, sustain: Sustain, history: &[String]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[2J\x1b[H")?;
    writeln!(out, "keyboard piano")?;

    // 左手は「キーの行」と「和音名の行」の2行で1段。右手は各段のキー行に並べる
    let set = &CHORD_SETS[chord_set];
    // セット名は全角なので、表示幅 (1文字2マス) で右手の列 (30マス目) まで埋める
    let padding = " ".repeat(13usize.saturating_sub(set.name.chars().count() * 2));
    writeln!(out, "  左手: 伴奏 [{}] {}{padding}右手: メロディ", chord_set + 1, set.name)?;
    let rows = CHORD_KEY_ROWS.iter().zip(&set.rows).zip(keymap::melody_rows());
    for ((chord_keys, chords), (melody_keys, start)) in rows {
        let chord_keys: String = chord_keys.iter().map(|&k| format!("{:<8}", k as char)).collect();
        let chord_names: String = chords.iter().map(|c| format!("{:<8}", c.name(key))).collect();
        let melody_keys: Vec<_> = melody_keys.iter().map(|&k| (k as char).to_string()).collect();
        let melody_names: Vec<_> = (start..start + melody_keys.len()).map(music::solfege).collect();
        writeln!(out, "    {chord_keys}  {} -> {}", melody_keys.join(" "), melody_names.join(" "))?;
        writeln!(out, "    {chord_names}")?;
    }
    writeln!(out)?;

    let keys: Vec<_> = MUSICAL_KEYS.iter().enumerate().map(|(i, k)| format!("{}:{}", i + 1, k.name)).collect();
    writeln!(out, "  1〜9               -> 調の切り替え ({})", keys.join(" "))?;
    match sustain {
        Sustain::UntilRelease => writeln!(out, "  Space (押しながら) -> メロディを半音上げる (#)")?,
        Sustain::Fixed(_) => writeln!(out, "  Space              -> 次のメロディ1音を半音上げる (#)")?,
    }
    match sustain {
        Sustain::UntilRelease => writeln!(out, "  左Cmd / Tab        -> 伴奏セットの切り替え")?,
        Sustain::Fixed(_) => writeln!(out, "  Tab                -> 伴奏セットの切り替え")?,
    }
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
