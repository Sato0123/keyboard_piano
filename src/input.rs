//! 入力: 端末から届くバイト列をキーイベントに変換する。
//! kitty protocol の有無による違いはここで吸収し、呼び出し側には同じ KeyEvent を返す

use std::io::{self, Read};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Char(u8),
    Escape,
    Backspace,
    Tab,
    /// 左 Cmd 単体 (kitty protocol で修飾キー自体も報告される場合のみ届く)
    LeftCmd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

/// 端末から届くイベント
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Key(KeyEvent),
    /// 端末ウィンドウのフォーカスが入った (true) / 外れた (false)
    Focus(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
    pub key: Key,
    pub ctrl: bool,
    /// Cmd を押しながら (kitty protocol 有効時のみ分かる)
    pub cmd: bool,
    pub kind: KeyEventKind,
}

pub struct InputReader<R> {
    reader: R,
    /// true なら kitty protocol 有効 (押下/離上が CSI u で届く)
    kitty: bool,
}

impl<R: Read> InputReader<R> {
    pub fn new(reader: R, kitty: bool) -> Self {
        Self { reader, kitty }
    }

    /// 次のイベントを待って返す。入力が閉じたら None
    pub fn next_event(&mut self) -> io::Result<Option<InputEvent>> {
        loop {
            let Some(byte) = self.read_byte()? else {
                return Ok(None);
            };
            let event = if !self.kitty {
                Some(InputEvent::Key(decode_legacy(byte)))
            } else if byte == 0x1b {
                self.read_kitty_sequence()?
            } else {
                // kitty 有効中でも端末によっては Backspace / Ctrl+H が従来通りのバイトで届く。
                // それ以外の素のバイトは離上が来ないので無視する
                Some(decode_legacy(byte))
                    .filter(|e| matches!(e.key, Key::Backspace | Key::Char(b'h')))
                    .map(InputEvent::Key)
            };
            if let Some(event) = event {
                return Ok(Some(event));
            }
        }
    }

    fn read_byte(&mut self) -> io::Result<Option<u8>> {
        let mut byte = [0u8; 1];
        Ok((self.reader.read(&mut byte)? == 1).then_some(byte[0]))
    }

    /// ESC の後ろを読み、`CSI ... u` ならキーイベント、`CSI I` / `CSI O` ならフォーカス変化にする
    fn read_kitty_sequence(&mut self) -> io::Result<Option<InputEvent>> {
        let mut byte = [0u8; 1];
        self.reader.read_exact(&mut byte)?;
        if byte[0] != b'[' {
            return Ok(None);
        }

        let mut params = Vec::new();
        let final_byte = loop {
            self.reader.read_exact(&mut byte)?;
            if (0x40..=0x7e).contains(&byte[0]) {
                break byte[0];
            }
            params.push(byte[0]);
        };
        Ok(match (params.is_empty(), final_byte) {
            (true, b'I') => Some(InputEvent::Focus(true)),
            (true, b'O') => Some(InputEvent::Focus(false)),
            _ => parse_kitty_key(&params, final_byte).map(InputEvent::Key),
        })
    }
}

/// 従来の端末入力の1バイトを解釈する。離上は分からないので常に Press
fn decode_legacy(byte: u8) -> KeyEvent {
    let (key, ctrl) = match byte {
        0x1b => (Key::Escape, false),
        0x7f => (Key::Backspace, false),
        0x09 => (Key::Tab, false),
        // Ctrl+A〜Ctrl+Z は 0x01〜0x1a で届く (Ctrl+C = 0x03, Ctrl+H = 0x08, Ctrl+L = 0x0c)
        0x01..=0x1a => (Key::Char(b'a' + byte - 1), true),
        _ => (Key::Char(byte), false),
    };
    KeyEvent { key, ctrl, cmd: false, kind: KeyEventKind::Press }
}

/// `CSI` の後ろ(パラメータ部と終端文字)を kitty protocol のキーイベントとして解釈する。
/// 形式: `keycode[:alternates] ; modifiers[:event] [; text] u`
fn parse_kitty_key(params: &[u8], final_byte: u8) -> Option<KeyEvent> {
    if final_byte != b'u' {
        return None;
    }
    let params = std::str::from_utf8(params).ok()?;
    let mut fields = params.split(';');

    let code: u32 = fields.next()?.split(':').next()?.parse().ok()?;
    let key = match code {
        9 => Key::Tab,
        27 => Key::Escape,
        127 => Key::Backspace,
        57444 => Key::LeftCmd,
        _ => Key::Char(u8::try_from(code).ok()?),
    };

    let mut modifier_field = fields.next().unwrap_or("1").split(':');
    let modifiers: u32 = modifier_field.next().unwrap_or("1").parse().unwrap_or(1);
    let kind = match modifier_field.next().unwrap_or("1") {
        "2" => KeyEventKind::Repeat,
        "3" => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    };

    // modifiers は 1 + ビットフラグ (shift=1, alt=2, ctrl=4, super(Cmd)=8)
    let flags = modifiers.saturating_sub(1);
    Some(KeyEvent {
        key,
        ctrl: flags & 0b100 != 0,
        cmd: flags & 0b1000 != 0,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(key: Key, ctrl: bool, kind: KeyEventKind) -> KeyEvent {
        KeyEvent { key, ctrl, cmd: false, kind }
    }

    #[test]
    fn parses_kitty_key_events() {
        use KeyEventKind::*;
        assert_eq!(parse_kitty_key(b"97", b'u'), Some(event(Key::Char(b'a'), false, Press)));
        assert_eq!(parse_kitty_key(b"59;1:3", b'u'), Some(event(Key::Char(b';'), false, Release)));
        assert_eq!(parse_kitty_key(b"99;5", b'u'), Some(event(Key::Char(b'c'), true, Press)));
        assert_eq!(parse_kitty_key(b"127;1:2", b'u'), Some(event(Key::Backspace, false, Repeat)));
        // 左 Cmd を押すと、それ自体が super 修飾付きで届く
        let left_cmd = KeyEvent { key: Key::LeftCmd, ctrl: false, cmd: true, kind: Press };
        assert_eq!(parse_kitty_key(b"57444;9", b'u'), Some(left_cmd));
        assert_eq!(parse_kitty_key(b"9", b'u'), Some(event(Key::Tab, false, Press)));
        assert_eq!(parse_kitty_key(b"1;1:3", b'A'), None);
    }

    #[test]
    fn decodes_legacy_bytes() {
        use KeyEventKind::Press;
        assert_eq!(decode_legacy(b'a'), event(Key::Char(b'a'), false, Press));
        assert_eq!(decode_legacy(0x03), event(Key::Char(b'c'), true, Press));
        assert_eq!(decode_legacy(0x0c), event(Key::Char(b'l'), true, Press));
        assert_eq!(decode_legacy(0x7f), event(Key::Backspace, false, Press));
        assert_eq!(decode_legacy(0x09), event(Key::Tab, false, Press));
    }

    #[test]
    fn reader_skips_unknown_sequences() {
        let input: &[u8] = b"\x1b[1;1:3A\x1b[97u\x1b[O\x1b[97;1:3u";
        let mut reader = InputReader::new(input, true);
        let a = |kind| Some(InputEvent::Key(event(Key::Char(b'a'), false, kind)));
        assert_eq!(reader.next_event().unwrap(), a(KeyEventKind::Press));
        assert_eq!(reader.next_event().unwrap(), Some(InputEvent::Focus(false)));
        assert_eq!(reader.next_event().unwrap(), a(KeyEventKind::Release));
        assert_eq!(reader.next_event().unwrap(), None);
    }
}
