//! 音楽理論: 音階・調・和音と、それらから周波数や名前を求める計算

/// 度数 0 の音 (C3) の周波数とオクターブ番号
const BASE_FREQ: f32 = 130.81;
const BASE_OCTAVE: usize = 3;

/// メジャースケールの各度数の半音オフセット
const MAJOR_SCALE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const SOLFEGE: [&str; 7] = ["ド", "レ", "ミ", "ファ", "ソ", "ラ", "シ"];
/// 各度数の上に積んだ3和音 / 4和音の種類 (メジャースケールなので調によらず同じ)
const TRIAD_QUALITIES: [&str; 7] = ["", "m", "m", "", "", "m", "dim"];
const SEVENTH_QUALITIES: [&str; 7] = ["maj7", "m7", "m7", "maj7", "7", "m7", "m7♭5"];
const SHARP_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLAT_NAMES: [&str; 12] = ["C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B"];

/// 調。ドレミ (度数) をどの高さで鳴らすかを決める
pub struct MusicalKey {
    pub name: &'static str,
    /// C からの移調量 (半音)。音域が上がりすぎないよう G 以上は下へずらす
    offset: i32,
    /// 和音名を ♭ で表記するか
    flats: bool,
}

/// 数字キー 1〜9 で切り替える調
pub const MUSICAL_KEYS: [MusicalKey; 9] = [
    MusicalKey { name: "C", offset: 0, flats: false },
    MusicalKey { name: "D", offset: 2, flats: false },
    MusicalKey { name: "E", offset: 4, flats: false },
    MusicalKey { name: "F", offset: 5, flats: true },
    MusicalKey { name: "G", offset: -5, flats: false },
    MusicalKey { name: "A", offset: -3, flats: false },
    MusicalKey { name: "B", offset: -1, flats: false },
    MusicalKey { name: "B♭", offset: -2, flats: true },
    MusicalKey { name: "E♭", offset: 3, flats: true },
];

/// 度数 (0 = C3 のド、7 = C4 のド …) を調に合わせた周波数にする
pub fn frequency(degree: usize, key: &MusicalKey) -> f32 {
    let semitones = 12 * (degree / 7) as i32 + MAJOR_SCALE[degree % 7] + key.offset;
    BASE_FREQ * 2f32.powf(semitones as f32 / 12.0)
}

/// 度数の階名 (オクターブ無し)。例: 9 -> "ミ"
pub fn solfege(degree: usize) -> &'static str {
    SOLFEGE[degree % 7]
}

/// 度数の階名 + オクターブ番号。例: 9 -> "ミ4"
pub fn note_name(degree: usize) -> String {
    format!("{}{}", solfege(degree), BASE_OCTAVE + degree / 7)
}

/// 度数を根音として、スケール上で1つ飛ばしに積んだ和音
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chord {
    root: usize,
    /// true なら7度も重ねた4和音
    seventh: bool,
}

impl Chord {
    pub const fn triad(root: usize) -> Self {
        Self { root, seventh: false }
    }

    pub const fn seventh(root: usize) -> Self {
        Self { root, seventh: true }
    }

    /// 構成音の度数 (根音・3度・5度・(7度))
    pub fn degrees(&self) -> Vec<usize> {
        let tones = if self.seventh { 4 } else { 3 };
        (0..tones).map(|i| self.root + 2 * i).collect()
    }

    /// 調に合わせた和音名。例: G調の ii -> "Am"、V7 -> "D7"
    pub fn name(&self, key: &MusicalKey) -> String {
        let degree = self.root % 7;
        let root = (MAJOR_SCALE[degree] + key.offset).rem_euclid(12) as usize;
        let names = if key.flats { &FLAT_NAMES } else { &SHARP_NAMES };
        let quality = if self.seventh { SEVENTH_QUALITIES[degree] } else { TRIAD_QUALITIES[degree] };
        format!("{}{}", names[root], quality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_follows_degree_and_key() {
        let c = &MUSICAL_KEYS[0];
        let g = &MUSICAL_KEYS[4];
        assert!((frequency(12, c) - 440.0).abs() < 0.1); // ラ4 = A4
        assert!((frequency(7, g) - 196.0).abs() < 0.1); // G調のド4 = G3
    }

    #[test]
    fn names_notes_and_chords() {
        let g = &MUSICAL_KEYS[4];
        assert_eq!(note_name(9), "ミ4");
        assert_eq!(Chord::triad(0).name(g), "G");
        assert_eq!(Chord::triad(6).name(g), "F#dim");
        assert_eq!(Chord::seventh(4).name(g), "D7");
        assert_eq!(Chord::seventh(3).name(&MUSICAL_KEYS[7]), "E♭maj7");
    }

    #[test]
    fn stacks_chord_tones_in_thirds() {
        assert_eq!(Chord::triad(4).degrees(), vec![4, 6, 8]);
        assert_eq!(Chord::seventh(4).degrees(), vec![4, 6, 8, 10]);
    }
}
