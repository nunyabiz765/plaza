//! Computer-keyboard piano (tracker layout) and MIDI byte parsing.

/// Semitone offset from the base C for a typed character. Two octaves plus a
/// few extra keys: bottom row Z..M is the lower octave (S D G H J the black
/// keys), Q..I the upper (2 3 5 6 7), and I 9 O 0 P run on past it.
pub fn semitone(c: char) -> Option<u8> {
    let s = match c.to_ascii_lowercase() {
        'z' => 0, 's' => 1, 'x' => 2, 'd' => 3, 'c' => 4, 'v' => 5, 'g' => 6,
        'b' => 7, 'h' => 8, 'n' => 9, 'j' => 10, 'm' => 11, ',' => 12, 'l' => 13, '.' => 14,
        'q' => 12, '2' => 13, 'w' => 14, '3' => 15, 'e' => 16, 'r' => 17, '5' => 18,
        't' => 19, '6' => 20, 'y' => 21, '7' => 22, 'u' => 23, 'i' => 24, '9' => 25,
        'o' => 26, '0' => 27, 'p' => 28,
        _ => return None,
    };
    Some(s)
}

/// Label printed on the on-screen key for each of the 25 shown semitones.
pub const KEY_LABELS: [&str; 25] = [
    "Z", "S", "X", "D", "C", "V", "G", "B", "H", "N", "J", "M", "Q", "2", "W", "3", "E", "R", "5", "T", "6", "Y",
    "7", "U", "I",
];

pub const MIN_OCTAVE: i32 = 1;
pub const MAX_OCTAVE: i32 = 7;

pub fn base_note(octave: i32) -> u8 {
    (12 * (octave.clamp(MIN_OCTAVE, MAX_OCTAVE) + 1)) as u8
}

pub fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
    format!("{}{}", NAMES[(n % 12) as usize], n as i32 / 12 - 1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Midi {
    NoteOn(u8, u8),
    NoteOff(u8),
    /// -1..1
    Bend(f32),
    ModWheel(f32),
    Sustain(bool),
    AllNotesOff,
}

/// Parses one channel-voice message (any channel). Unknown messages are None.
pub fn parse_midi(b: &[u8]) -> Option<Midi> {
    let status = *b.first()?;
    let d1 = b.get(1).copied().unwrap_or(0);
    let d2 = b.get(2).copied().unwrap_or(0);
    match status & 0xF0 {
        0x90 if d2 > 0 => Some(Midi::NoteOn(d1 & 0x7F, d2 & 0x7F)),
        0x90 | 0x80 => Some(Midi::NoteOff(d1 & 0x7F)),
        0xE0 => {
            let v = ((d2 as i32) << 7 | d1 as i32) - 8192;
            Some(Midi::Bend(v as f32 / 8192.0))
        }
        0xB0 => match d1 {
            1 => Some(Midi::ModWheel(d2 as f32 / 127.0)),
            64 => Some(Midi::Sustain(d2 >= 64)),
            120 | 123 => Some(Midi::AllNotesOff),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_two_chromatic_octaves() {
        let mut seen = [false; 25];
        for c in "zsxdcvgbhnjmq2w3er5t6y7ui".chars() {
            seen[semitone(c).unwrap() as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(semitone('Q'), Some(12));
        assert_eq!(semitone('a'), None);
        for (i, l) in KEY_LABELS.iter().enumerate() {
            assert_eq!(semitone(l.chars().next().unwrap()), Some(i as u8));
        }
    }

    #[test]
    fn midi_parsing() {
        assert_eq!(parse_midi(&[0x90, 60, 100]), Some(Midi::NoteOn(60, 100)));
        assert_eq!(parse_midi(&[0x93, 60, 0]), Some(Midi::NoteOff(60)));
        assert_eq!(parse_midi(&[0x80, 60, 40]), Some(Midi::NoteOff(60)));
        assert_eq!(parse_midi(&[0xE0, 0, 64]), Some(Midi::Bend(0.0)));
        assert_eq!(parse_midi(&[0xE0, 0, 0]), Some(Midi::Bend(-1.0)));
        assert_eq!(parse_midi(&[0xB0, 64, 127]), Some(Midi::Sustain(true)));
        assert_eq!(parse_midi(&[0xF8]), None);
        assert_eq!(parse_midi(&[]), None);
    }

    #[test]
    fn names() {
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(69), "A4");
        assert_eq!(base_note(4), 60);
    }
}
