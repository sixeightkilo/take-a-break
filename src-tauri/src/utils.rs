//! Miscellaneous utilities
mod utils {
    #[cfg(target_os = "macos")]
    use plist::Value;
    use std::fmt;
    #[cfg(target_os = "macos")]
    use std::io::Cursor;
    use std::process::Command;

    use dirs;
    use log::LevelFilter;
    use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
    use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
    use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
    use log4rs::append::rolling_file::RollingFileAppender;
    use log4rs::{
        config::{Appender, Config, Root},
        encode::pattern::PatternEncoder,
    };
    use std::path::PathBuf;

    pub enum LockedState {
        Locked,
        Unlocked,
        Unknown,
    }

    /// Checks whether the current desktop session is locked.
    pub fn is_locked() -> LockedState {
        #[cfg(target_os = "linux")]
        {
            // Check every available source: logind may report an unset hint even
            // when the desktop's own screen saver knows the session is locked.
            let mut found_unlocked = false;
            let checks: [(&str, &[&str]); 3] = [
                (
                    "loginctl",
                    &["show-session", "self", "-p", "LockedHint", "--value"],
                ),
                (
                    "dbus-send",
                    &[
                        "--session",
                        "--print-reply",
                        "--dest=org.freedesktop.ScreenSaver",
                        "/org/freedesktop/ScreenSaver",
                        "org.freedesktop.ScreenSaver.GetActive",
                    ],
                ),
                (
                    "dbus-send",
                    &[
                        "--session",
                        "--print-reply",
                        "--dest=org.gnome.ScreenSaver",
                        "/org/gnome/ScreenSaver",
                        "org.gnome.ScreenSaver.GetActive",
                    ],
                ),
            ];

            for (program, args) in checks {
                match command_locked_state(program, args) {
                    Some(LockedState::Locked) => return LockedState::Locked,
                    Some(LockedState::Unlocked) => found_unlocked = true,
                    _ => {}
                }
            }
            return if found_unlocked {
                LockedState::Unlocked
            } else {
                LockedState::Unknown
            };
        }

        #[cfg(target_os = "macos")]
        {
            let buf = Command::new("ioreg")
                .args(&["-n", "Root", "-d1", "-a"])
                .output();

            let output;

            match buf {
                Ok(buf) => {
                    if buf.status.success() {
                        output = buf.stdout;
                    } else {
                        return LockedState::Unknown;
                    }
                }
                Err(_) => return LockedState::Unknown,
            }

            let v: Value;
            if let Ok(result) = Value::from_reader(Cursor::new(output)) {
                v = result;
            } else {
                return LockedState::Unknown;
            }

            let is_locked = v
                .as_dictionary()
                .and_then(|dict| dict.get("IOConsoleLocked"));

            match is_locked {
                Some(l) => {
                    if let Some(b) = l.as_boolean() {
                        return if b {
                            LockedState::Locked
                        } else {
                            LockedState::Unlocked
                        };
                    }
                    return LockedState::Unknown;
                }
                None => LockedState::Unknown,
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            LockedState::Unknown
        }
    }

    #[cfg(target_os = "linux")]
    fn command_locked_state(program: &str, args: &[&str]) -> Option<LockedState> {
        let output = Command::new(program).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let output = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_ascii_lowercase();
        let value = output.split_whitespace().last().unwrap_or("");
        match value {
            "yes" | "true" | "1" => Some(LockedState::Locked),
            "no" | "false" | "0" => Some(LockedState::Unlocked),
            _ => None,
        }
    }

    impl fmt::Debug for LockedState {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                LockedState::Locked => write!(f, "Locked"),
                LockedState::Unlocked => write!(f, "Unlocked"),
                LockedState::Unknown => write!(f, "Unknown"),
            }
        }
    }

    // Logger settings.Sends all debug prints to <app_dir>/debug.log
    // Log file rotated after 1M.
    pub fn setup_logger() {
        //setup rotation
        let file_name = get_log_file_name();
        println!("file_name: {}", file_name);
        let window_size = 1; // log0, log1, log2
        let fixed_window_roller = FixedWindowRoller::builder()
            .build(&(file_name.clone() + "{}"), window_size)
            .unwrap();
        let size_limit = 1 * 1024 * 1024; //1mb
        let size_trigger = SizeTrigger::new(size_limit);
        let compound_policy =
            CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));

        let logfile = RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S%.3f)} [{l}] - {m}{n}",
            )))
            .build(file_name, Box::new(compound_policy))
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Debug),
            )
            .unwrap();

        log4rs::init_config(config).unwrap();
    }

    pub fn get_log_file_name() -> String {
        let mut path = get_app_dir().expect("get_app_dir() returned None");

        std::fs::create_dir_all(&path).expect("failed to create app dir");

        path.push("debug.log");
        path.to_string_lossy().to_string()
    }

    pub fn get_settings_file_name() -> String {
        if let Some(mut path) = get_app_dir() {
            path.push("settings.json");
            return path.to_string_lossy().to_string();
        }

        return "".to_string();
    }

    fn get_app_dir() -> Option<PathBuf> {
        #[cfg(feature = "debug")]
        {
            if let Some(home_dir) = dirs::home_dir() {
                let mut path = PathBuf::new();
                path.push(home_dir);
                path.push("Library");
                path.push("com.68kilo.tab_debug"); // Debug-specific path
                return Some(path);
            }
        }

        #[cfg(not(feature = "debug"))]
        {
            if let Some(home_dir) = dirs::home_dir() {
                let mut path = PathBuf::new();
                path.push(home_dir);
                path.push("Library");
                path.push("com.68kilo.tab"); // Release-specific path
                return Some(path);
            }
        }

        // Default path if no features are enabled
        None
    }
}

pub use utils::*;
