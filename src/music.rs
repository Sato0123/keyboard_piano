//! 音楽理論: 音階・調・和音と、それらから周波数や名前を求める計算

/// 度数 0 の音 (C3) の周波数とオクターブ番号
const BASE_FREQ: f32 = 130.81;
const BASE_OCTAVE: usize = 3;

/// メジャースケールの各度数の半音オフセット
const MAJOR_SCALE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const SOLFEGE: [&str; 7] = ["ド", "レ", "ミ", "ファ", "ソ", "ラ", "シ"];
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

/// 度数 (0 = C3 のド、7 = C4 のド …) を調に合わせた周波数にする。sharp なら半音上げる
pub fn frequency(degree: usize, sharp: bool, key: &MusicalKey) -> f32 {
    semitones_to_frequency(semitones(degree, key) + i32::from(sharp))
}

/// 度数が C3 から何半音上か (調の移調込み)
fn semitones(degree: usize, key: &MusicalKey) -> i32 {
    12 * (degree / 7) as i32 + MAJOR_SCALE[degree % 7] + key.offset
}

fn semitones_to_frequency(semitones: i32) -> f32 {
    BASE_FREQ * 2f32.powf(semitones as f32 / 12.0)
}

/// 度数の階名 (オクターブ無し)。例: 9 -> "ミ"
pub fn solfege(degree: usize) -> &'static str {
    SOLFEGE[degree % 7]
}

/// 度数の階名 + オクターブ番号。例: 9 -> "ミ4"、半音上げなら "ミ#4"
pub fn note_name(degree: usize, sharp: bool) -> String {
    format!("{}{}{}", solfege(degree), if sharp { "#" } else { "" }, BASE_OCTAVE + degree / 7)
}

/// 和音の種類。根音からの半音間隔で決まるので、調の外の和音 (III7 など) も作れる
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Major,
    Minor,
    Major7,
    Dominant7,
    Minor7,
}

impl Quality {
    fn intervals(self) -> &'static [i32] {
        match self {
            Quality::Major => &[0, 4, 7],
            Quality::Minor => &[0, 3, 7],
            Quality::Major7 => &[0, 4, 7, 11],
            Quality::Dominant7 => &[0, 4, 7, 10],
            Quality::Minor7 => &[0, 3, 7, 10],
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Quality::Major => "",
            Quality::Minor => "m",
            Quality::Major7 => "maj7",
            Quality::Dominant7 => "7",
            Quality::Minor7 => "m7",
        }
    }
}

/// 度数を根音とする和音
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chord {
    /// 根音の度数 (0〜6 なら C3 のオクターブ)
    root: usize,
    quality: Quality,
}

impl Chord {
    pub const fn new(root: usize, quality: Quality) -> Self {
        Self { root, quality }
    }

    /// 調に合わせた構成音の周波数
    pub fn frequencies(&self, key: &MusicalKey) -> Vec<f32> {
        let root = semitones(self.root, key);
        self.quality
            .intervals()
            .iter()
            .map(|interval| semitones_to_frequency(root + interval))
            .collect()
    }

    /// 調に合わせた和音名。例: G調の vim7 -> "Em7"、III7 -> "B7"
    pub fn name(&self, key: &MusicalKey) -> String {
        let root = semitones(self.root, key).rem_euclid(12) as usize;
        let names = if key.flats { &FLAT_NAMES } else { &SHARP_NAMES };
        format!("{}{}", names[root], self.quality.suffix())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_follows_degree_and_key() {
        let c = &MUSICAL_KEYS[0];
        let g = &MUSICAL_KEYS[4];
        assert!((frequency(12, false, c) - 440.0).abs() < 0.1); // ラ4 = A4
        assert!((frequency(7, false, g) - 196.0).abs() < 0.1); // G調のド4 = G3
        assert!((frequency(10, true, c) - 369.99).abs() < 0.1); // ファ#4 = F#4
    }

    #[test]
    fn names_notes_and_chords() {
        let g = &MUSICAL_KEYS[4];
        assert_eq!(note_name(9, false), "ミ4");
        assert_eq!(note_name(10, true), "ファ#4");
        assert_eq!(Chord::new(0, Quality::Major).name(g), "G");
        assert_eq!(Chord::new(2, Quality::Dominant7).name(g), "B7");
        assert_eq!(Chord::new(4, Quality::Minor7).name(&MUSICAL_KEYS[0]), "Gm7");
        assert_eq!(Chord::new(3, Quality::Major7).name(&MUSICAL_KEYS[7]), "E♭maj7");
    }

    #[test]
    fn builds_chord_tones_from_intervals() {
        let c = &MUSICAL_KEYS[0];
        // E7 = E3 G#3 B3 D4
        let e7 = Chord::new(2, Quality::Dominant7).frequencies(c);
        let expected = [164.81, 207.65, 246.94, 293.66];
        assert_eq!(e7.len(), 4);
        for (f, e) in e7.iter().zip(expected) {
            assert!((f - e).abs() < 0.1, "{f} != {e}");
        }
    }
}
