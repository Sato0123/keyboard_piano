//! キー配置: どのキーがメロディのどの音・伴奏のどの和音か

use crate::music::{Chord, Quality};

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

/// 左手の和音ボタンのキー (上の段から)
pub const CHORD_KEY_ROWS: [&[u8; 3]; 3] = [b"qwe", b"asd", b"zxc"];

/// 和音ボタンに割り当てる和音の組。左 Cmd / Tab で切り替える
pub struct ChordSet {
    pub name: &'static str,
    /// CHORD_KEY_ROWS と同じ並び
    pub rows: [[Chord; 3]; 3],
}

const fn chord(root: usize, quality: Quality) -> Chord {
    Chord::new(root, quality)
}

use Quality::*;

pub const CHORD_SETS: [ChordSet; 2] = [
    // アコーディオンのように中段に IV I V、その真下に関係調の ii vi iii、
    // 真上に IV I V のセブンスを並べる
    ChordSet {
        name: "基本",
        rows: [
            [chord(3, Major7), chord(0, Major7), chord(4, Dominant7)], // IVmaj7 Imaj7 V7
            [chord(3, Major), chord(0, Major), chord(4, Major)],       // IV I V
            [chord(1, Minor), chord(5, Minor), chord(2, Minor)],       // ii vi iii
        ],
    },
    // 基本セットと同じ位置に同じ根音のセブンスを置き、上段に調の外の和音を足す。
    // 丸サ進行 (IVmaj7 -> III7 -> vim7 -> vm7 I7) は a -> c -> x -> q w で弾ける
    ChordSet {
        name: "セブンス",
        rows: [
            [chord(4, Minor7), chord(0, Dominant7), chord(1, Dominant7)], // vm7 I7 II7
            [chord(3, Major7), chord(0, Major7), chord(4, Dominant7)],    // IVmaj7 Imaj7 V7
            [chord(1, Minor7), chord(5, Minor7), chord(2, Dominant7)],    // iim7 vim7 III7
        ],
    },
];

/// set は CHORD_SETS の添字
pub fn note_for(key: u8, set: usize) -> Option<Note> {
    if let Some(i) = MELODY_KEYS.iter().position(|&k| k == key) {
        return Some(Note::Melody(MELODY_START_DEGREE + i));
    }
    CHORD_KEY_ROWS
        .iter()
        .zip(&CHORD_SETS[set].rows)
        .find_map(|(keys, chords)| keys.iter().position(|&k| k == key).map(|i| Note::Chord(chords[i])))
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
        assert_eq!(note_for(b'v', 0), Some(Note::Melody(7)));
        assert_eq!(note_for(b'f', 1), Some(Note::Melody(12)));
        assert_eq!(note_for(b'p', 0), Some(Note::Melody(21)));
        assert_eq!(note_for(b's', 0), Some(Note::Chord(chord(0, Major))));
        assert_eq!(note_for(b'x', 0), Some(Note::Chord(chord(5, Minor))));
        assert_eq!(note_for(b'e', 0), Some(Note::Chord(chord(4, Dominant7))));
        assert_eq!(note_for(b'c', 1), Some(Note::Chord(chord(2, Dominant7))));
        for unused in [b'y', b'h', b'n', b'g', b'b', b't'] {
            assert_eq!(note_for(unused, 0), None);
        }
    }

    #[test]
    fn second_set_plays_marusa_progression() {
        let c = &crate::music::MUSICAL_KEYS[0];
        let names: Vec<_> = b"acxqw"
            .iter()
            .map(|&k| match note_for(k, 1) {
                Some(Note::Chord(chord)) => chord.name(c),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(names, ["Fmaj7", "E7", "Am7", "Gm7", "C7"]);
    }

    #[test]
    fn melody_rows_go_top_to_bottom() {
        let rows: Vec<_> = melody_rows().collect();
        assert_eq!(rows[0], (&b"ruiop"[..], 17));
        assert_eq!(rows[2], (&b"vm,./"[..], 7));
    }
}
