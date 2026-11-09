mod player {
    use crate::utils::*;
    use rodio::{source::SineWave, source::Source, Decoder, OutputStream, Sink};
    use std::io::Cursor;
    use std::time::Duration;

    #[cfg(feature = "debug")]
    const START_REPEATS: usize = 1;
    #[cfg(not(feature = "debug"))]
    const START_REPEATS: usize = 5;

    pub fn play() {
        play_sound(false);
    }
    pub fn play_end() {
        play_sound(true);
    }

    fn play_sound(end: bool) {
        if matches!(is_locked(), LockedState::Locked) {
            return;
        }
        let Ok((_stream, handle)) = OutputStream::try_default() else {
            return;
        };
        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };
        if end {
            for _ in 0..4 {
                sink.append(
                    SineWave::new(880.0)
                        .amplify(0.2)
                        .take_duration(Duration::from_millis(180)),
                );
                sink.append(
                    SineWave::new(880.0)
                        .amplify(0.0)
                        .take_duration(Duration::from_millis(150)),
                );
            }
        } else {
            for _ in 0..START_REPEATS {
                let Ok(source) = Decoder::new(Cursor::new(include_bytes!("beep.mp3"))) else {
                    return;
                };
                // Preserve the existing one-beep-per-second start pattern.
                sink.append(source.take_duration(Duration::from_secs(1)));
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        sink.sleep_until_end();
    }
}

pub use player::*;
