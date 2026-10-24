//! 端末設定: 非カノニカルモードと kitty keyboard protocol の有効化・後始末

use std::io::{self, Read, Write};
use std::time::Duration;

/// 生成時に端末を非カノニカル・エコー無しに切り替え、Drop で元の設定に戻す
pub struct RawMode {
    original: libc::termios,
    raw: libc::termios,
}

impl RawMode {
    pub fn enable() -> io::Result<Self> {
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut raw = original;
        // ISIG も外し、Ctrl+C はシグナルではなく 0x03 として受け取る (端末設定を確実に戻すため)
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        let mode = Self { original, raw };
        mode.set_read_timeout(None)?;
        Ok(mode)
    }

    /// None: 1バイト来るまでブロック / Some: 最大その時間(0.1秒単位)待って諦める
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let mut t = self.raw;
        match timeout {
            None => {
                t.c_cc[libc::VMIN] = 1;
                t.c_cc[libc::VTIME] = 0;
            }
            Some(d) => {
                t.c_cc[libc::VMIN] = 0;
                t.c_cc[libc::VTIME] = (d.as_millis() / 100).clamp(1, 255) as libc::cc_t;
            }
        }
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &t) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original);
        }
    }
}

/// kitty keyboard protocol を有効にし、Drop で元に戻す。
/// 有効中はキーの押下/リピート/離上がすべて `CSI keycode ; modifiers:event u` で届く
pub struct KittyKeyboard;

impl KittyKeyboard {
    /// 1 = disambiguate, 2 = report event types, 8 = report all keys as escape codes
    const FLAGS: u8 = 1 | 2 | 8;

    /// 対応端末なら有効にして返す。`CSI ? u` (現在のフラグ問い合わせ) の後に
    /// 必ず返事が来る `CSI c` (Primary Device Attributes) を送り、
    /// DA1 の返事より前に `CSI ? flags u` が来れば対応している
    pub fn enable(raw: &RawMode, stdin: &mut impl Read, stdout: &mut impl Write) -> io::Result<Option<Self>> {
        stdout.write_all(b"\x1b[?u\x1b[c")?;
        stdout.flush()?;

        raw.set_read_timeout(Some(Duration::from_millis(500)))?;
        let mut response = Vec::new();
        let mut byte = [0u8; 1];
        while stdin.read(&mut byte)? == 1 {
            response.push(byte[0]);
            if byte[0] == b'c' && response.windows(3).any(|w| w == b"\x1b[?") {
                break;
            }
        }
        raw.set_read_timeout(None)?;

        let supported = response
            .split(|&b| b == 0x1b)
            .any(|seq| seq.starts_with(b"[?") && seq.ends_with(b"u"));
        if !supported {
            return Ok(None);
        }

        write!(stdout, "\x1b[>{}u", Self::FLAGS)?;
        stdout.flush()?;
        Ok(Some(Self))
    }
}

impl Drop for KittyKeyboard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[<u");
        let _ = stdout.flush();
    }
}

/// フォーカス通知を有効にし、Drop で元に戻す。
/// 有効中は端末ウィンドウのフォーカスが変わるたびに `CSI I` (入) / `CSI O` (外) が届く
pub struct FocusReporting;

impl FocusReporting {
    pub fn enable(stdout: &mut impl Write) -> io::Result<Self> {
        stdout.write_all(b"\x1b[?1004h")?;
        stdout.flush()?;
        Ok(Self)
    }
}

impl Drop for FocusReporting {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[?1004l");
        let _ = stdout.flush();
    }
}
