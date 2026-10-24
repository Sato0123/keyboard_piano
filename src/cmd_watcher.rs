//! 左 Cmd の監視: iTerm2 は Cmd 単体の押下を端末に流さないため、
//! macOS のキー状態を直接ポーリングして「左 Cmd だけを軽く押して離した」を検出する

use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// これより長く押していたらタップとみなさない
const TAP_MAX: Duration = Duration::from_millis(400);

/// 左 Cmd のタップを検出するたびに `on_tap()` の値を送るスレッドを起動する
pub fn spawn<T: Send + 'static>(tx: Sender<T>, on_tap: fn() -> T) {
    thread::spawn(move || {
        let mut detector = TapDetector::default();
        loop {
            let tapped = detector.update(Instant::now(), macos::left_cmd_down(), macos::any_other_key_down);
            if tapped && tx.send(on_tap()).is_err() {
                return; // 受け取り側が終了した
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
}

#[derive(Default)]
struct TapDetector {
    /// 左 Cmd を押し始めた時刻 (離している間は None)
    down_since: Option<Instant>,
    /// 押している間に他のキーも押された (Cmd+C などのショートカット)
    combined: bool,
}

impl TapDetector {
    /// 1回分のポーリング結果を反映し、タップが完了したら true を返す。
    /// 他のキーの状態は Cmd を押している間だけ調べればよいので関数で受け取る
    fn update(&mut self, now: Instant, cmd_down: bool, other_key_down: impl Fn() -> bool) -> bool {
        if cmd_down {
            if self.down_since.is_none() {
                self.down_since = Some(now);
                self.combined = false;
            }
            self.combined |= other_key_down();
            false
        } else {
            match self.down_since.take() {
                Some(since) => !self.combined && now - since < TAP_MAX,
                None => false,
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    /// kCGEventSourceStateHIDSystemState: ハードウェアから見たキー状態
    const HID_SYSTEM_STATE: i32 = 1;
    /// 仮想キーコード (Carbon の kVK_*)
    const LEFT_COMMAND: u16 = 0x37;
    /// 0x36〜0x3F は修飾キー (右Cmd, 左Cmd, Shift, CapsLock, Option, Control, 右Shift, 右Option, 右Control, Fn)
    const MODIFIER_KEYS: std::ops::RangeInclusive<u16> = 0x36..=0x3f;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
    }

    fn key_down(key: u16) -> bool {
        unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, key) }
    }

    pub fn left_cmd_down() -> bool {
        key_down(LEFT_COMMAND)
    }

    pub fn any_other_key_down() -> bool {
        (0..0x80).filter(|k| !MODIFIER_KEYS.contains(k)).any(key_down)
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    pub fn left_cmd_down() -> bool {
        false
    }

    pub fn any_other_key_down() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(detector: &mut TapDetector, start: Instant, steps: &[(u64, bool, bool)]) -> Vec<bool> {
        steps
            .iter()
            .map(|&(ms, cmd, other)| detector.update(start + Duration::from_millis(ms), cmd, || other))
            .collect()
    }

    #[test]
    fn detects_short_tap() {
        let start = Instant::now();
        let results = run(&mut TapDetector::default(), start, &[(0, true, false), (100, true, false), (150, false, false)]);
        assert_eq!(results, [false, false, true]);
    }

    #[test]
    fn ignores_long_press_and_shortcuts() {
        let start = Instant::now();
        let long = run(&mut TapDetector::default(), start, &[(0, true, false), (500, false, false)]);
        assert_eq!(long, [false, false]);

        let shortcut = run(&mut TapDetector::default(), start, &[(0, true, false), (50, true, true), (100, false, false)]);
        assert_eq!(shortcut, [false, false, false]);
    }
}
