//! 発音: サイン波の音源と、それをオーディオ出力に流す仕組み

use std::error::Error;
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

const SAMPLE_RATE: u32 = 48_000;
const ATTACK: Duration = Duration::from_millis(5);
const RELEASE: Duration = Duration::from_millis(120);

/// 音をいつまで鳴らすか
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sustain {
    /// HeldNote::release() が呼ばれるまで
    UntilRelease,
    /// 決まった時間だけ (キーを離したことが分からない端末向け)
    Fixed(Duration),
}

pub struct Synth {
    sink: rodio::MixerDeviceSink,
}

impl Synth {
    pub fn open() -> Result<Self, Box<dyn Error>> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink()?;
        sink.log_on_drop(false);
        Ok(Self { sink })
    }

    /// 複数の周波数を同時に鳴らし始める。返り値で後から止められる
    pub fn play(&self, freqs: &[f32], amplitude: f32, sustain: Sustain) -> HeldNote {
        let released = Arc::new(AtomicBool::new(false));
        for &freq in freqs {
            self.sink.mixer().add(Voice::new(freq, amplitude, released.clone(), sustain));
        }
        HeldNote { released }
    }
}

/// 鳴らしている音へのハンドル
pub struct HeldNote {
    released: Arc<AtomicBool>,
}

impl HeldNote {
    /// フェードアウトして止める
    pub fn release(&self) {
        self.released.store(true, Ordering::Relaxed);
    }
}

fn samples(duration: Duration) -> u64 {
    (duration.as_secs_f64() * SAMPLE_RATE as f64) as u64
}

/// `released` が立つ(または Sustain::Fixed の時間に達する)まで鳴り続け、
/// その後 RELEASE かけてフェードアウトして終わるサイン波
struct Voice {
    phase: f32,
    phase_step: f32,
    amplitude: f32,
    level: f32,
    attack_step: f32,
    release_step: f32,
    released: Arc<AtomicBool>,
    auto_release: Option<u64>,
    elapsed: u64,
}

impl Voice {
    fn new(freq: f32, amplitude: f32, released: Arc<AtomicBool>, sustain: Sustain) -> Self {
        Self {
            phase: 0.0,
            phase_step: freq / SAMPLE_RATE as f32,
            amplitude,
            level: 0.0,
            attack_step: 1.0 / samples(ATTACK) as f32,
            release_step: 1.0 / samples(RELEASE) as f32,
            released,
            auto_release: match sustain {
                Sustain::UntilRelease => None,
                Sustain::Fixed(d) => Some(samples(d)),
            },
            elapsed: 0,
        }
    }

    fn is_released(&self) -> bool {
        self.released.load(Ordering::Relaxed)
            || self.auto_release.is_some_and(|n| self.elapsed >= n)
    }
}

impl Iterator for Voice {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.is_released() {
            self.level -= self.release_step;
            if self.level <= 0.0 {
                return None;
            }
        } else {
            self.level = (self.level + self.attack_step).min(1.0);
        }
        self.elapsed += 1;

        let sample = (self.phase * std::f32::consts::TAU).sin() * self.level * self.amplitude;
        self.phase = (self.phase + self.phase_step).fract();
        Some(sample)
    }
}

impl Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_sustains_until_released_then_fades_out() {
        let released = Arc::new(AtomicBool::new(false));
        let mut voice = Voice::new(440.0, 0.1, released.clone(), Sustain::UntilRelease);

        assert_eq!(voice.by_ref().take(SAMPLE_RATE as usize).count(), SAMPLE_RATE as usize);

        released.store(true, Ordering::Relaxed);
        let tail = voice.count() as u64;
        assert!(tail <= samples(RELEASE) + 1, "tail = {tail}");
    }

    #[test]
    fn fixed_sustain_ends_by_itself() {
        let released = Arc::new(AtomicBool::new(false));
        let voice = Voice::new(440.0, 0.1, released, Sustain::Fixed(Duration::from_millis(100)));
        let len = voice.count() as u64;
        assert!(len <= samples(Duration::from_millis(100)) + samples(RELEASE) + 1, "len = {len}");
    }
}
