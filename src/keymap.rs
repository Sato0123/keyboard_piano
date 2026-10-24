//! キー配置: どのキーがメロディのどの音・伴奏のどの和音か

use crate::music::Chord;

/// キーを押した時に鳴らすもの
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Note {
    /// メロディの単音 (度数。0 = C3)
    Melody(usize),
    /// 伴奏の和音
    Chord(Chord),
}

/// 右手のメロディ用キー。下の段から順に音階が続く (ド4〜ド6)。y h n は使わない
///   r u i o p  -> ファ ソ ラ シ ド
///   f j k l ;  -> ラ シ ド レ ミ
///   v m , . /  -> ド レ ミ ファ ソ
const MELODY_KEYS: &[u8; 15] = b"vm,./fjkl;ruiop";
const MELODY_ROW_LEN: usize = 5;
const MELODY_START_DEGREE: usize = 7; // C4

/// 左手の和音ボタン (上の段から)。アコーディオンのように中段に IV I V、
/// その真下に関係調の ii vi iii、真上に IV I V のセブンスを並べる
pub const CHORD_ROWS: [[(u8, Chord); 3]; 3] = [
    [(b'q', Chord::seventh(3)), (b'w', Chord::seventh(0)), (b'e', Chord::seventh(4))],
    [(b'a', Chord::triad(3)), (b's', Chord::triad(0)), (b'd', Chord::triad(4))],
    [(b'z', Chord::triad(1)), (b'x', Chord::triad(5)), (b'c', Chord::triad(2))],
];

pub fn note_for(key: u8) -> Option<Note> {
    if let Some(i) = MELODY_KEYS.iter().position(|&k| k == key) {
        return Some(Note::Melody(MELODY_START_DEGREE + i));
    }
    CHORD_ROWS
        .iter()
        .flatten()
        .find(|(k, _)| *k == key)
        .map(|&(_, chord)| Note::Chord(chord))
}

/// メロディの各段 (上の段から) を (キー列, 先頭の度数) で返す
pub fn melody_rows() -> impl Iterator<Item = (&'static [u8], usize)> {
    MELODY_KEYS
        .chunks(MELODY_ROW_LEN)
        .enumerate()
        .rev()
        .map(|(row, keys)| (keys, MELODY_START_DEGREE + row * MELODY_ROW_LEN))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_melody_and_chord_keys() {
        assert_eq!(note_for(b'v'), Some(Note::Melody(7)));
        assert_eq!(note_for(b'f'), Some(Note::Melody(12)));
        assert_eq!(note_for(b'p'), Some(Note::Melody(21)));
        assert_eq!(note_for(b's'), Some(Note::Chord(Chord::triad(0))));
        assert_eq!(note_for(b'x'), Some(Note::Chord(Chord::triad(5))));
        assert_eq!(note_for(b'e'), Some(Note::Chord(Chord::seventh(4))));
        for unused in [b'y', b'h', b'n', b'g', b'b', b't'] {
            assert_eq!(note_for(unused), None);
        }
    }

    #[test]
    fn melody_rows_go_top_to_bottom() {
        let rows: Vec<_> = melody_rows().collect();
        assert_eq!(rows[0], (&b"ruiop"[..], 17));
        assert_eq!(rows[2], (&b"vm,./"[..], 7));
    }
}
