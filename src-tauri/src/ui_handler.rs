//! Module to handle interactions with UI.
//! Receives commands from JS world and sends back events.
mod alarm_manager;
mod ui_handler {

    use super::alarm_manager::*;
    use crate::settings::Settings;
    use crate::utils::*;
    use chrono::Weekday;
    use tauri::Emitter;
    // use log::debug;
    use serde::ser::SerializeStruct;
    use serde::Deserialize;
    use serde::Serialize;
    use serde::Serializer;
    use serde_json::json;
    use serde_json::Value;
    use std::fmt;
    use std::fs::OpenOptions;
    use std::io::Read;
    use tauri::AppHandle;
    // use tauri::Manager;
    use tauri::Wry;
    use tokio::sync::broadcast;
    use tokio::sync::mpsc::Receiver;

    use tokio::sync::broadcast::Receiver as R;
    pub type BcastReceiver<T> = R<T>;

    use tokio::sync::broadcast::Sender as S;
    pub type BcastSender<T> = S<T>;

    const BCAST_CHANNEL_SIZE: usize = 10;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Rule {
        pub days: Vec<String>,
        pub from: usize,
        pub interval: usize,
        pub serial: usize,
        pub to: usize,
    }

    #[derive(Clone, Debug)]
    pub enum Payload {
        Rules(Vec<Rule>),
        BreakDuration(u64),
        Alarm(Option<Alarm>),
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum MessageType {
        //from the UI
        CmdStartup,
        CmdUpdateRules,
        CmdUpdateBreakDuration,
        EvtBreakDurationApplied,
        //For UI
        EvtRulesApplied,
        EvtStarted,

        //For alarm manager
        CmdUpdateAlarms,
        //From alarm manager
        EvtNextAlarm,
        EvtPlayingAlarm,
    }

    impl MessageType {
        pub fn from_str(typ: &str) -> Option<Self> {
            match typ {
                "cmd-update-rules" => Some(MessageType::CmdUpdateRules),
                "cmd-update-break-duration" => Some(MessageType::CmdUpdateBreakDuration),
                "cmd-startup" => Some(MessageType::CmdStartup),
                _ => None,
            }
        }
    }

    impl fmt::Display for MessageType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MessageType::EvtBreakDurationApplied => write!(f, "event-break-duration-applied"),
                MessageType::EvtNextAlarm => write!(f, "event-next-alarm"),
                MessageType::EvtStarted => write!(f, "event-started"),
                MessageType::EvtRulesApplied => write!(f, "event-rules-applied"),
                _ => write!(f, "not-implemented"),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Message {
        pub typ: MessageType,
        pub payload: Payload,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Alarm {
        pub day: Weekday,
        pub hour: usize,
        pub min: usize,
    }

    impl Serialize for Alarm {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("Alarm", 3)?;
            state.serialize_field("day", &format!("{:?}", self.day))?;
            state.serialize_field("hour", &self.hour)?;
            state.serialize_field("min", &self.min)?;
            state.end()
        }
    }

    pub struct UiHandler {
        ui_rx: Receiver<String>,
        win_handle: AppHandle<Wry>,

        //for managing communication with alarm manager
        am_tx: BcastSender<Message>,
        am_rx: BcastReceiver<Message>,
        rules: Vec<Rule>,
        break_duration: u64,
        prev_alarm: Option<Alarm>,
    }

    impl UiHandler {
        pub fn new(ui_rx: Receiver<String>, win_handle: AppHandle<Wry>) -> Self {
            let (am_tx, am_rx): (BcastSender<Message>, BcastReceiver<Message>) =
                broadcast::channel(BCAST_CHANNEL_SIZE);

            let settings = Self::read_settings();

            Self {
                ui_rx,
                win_handle,
                am_tx,
                am_rx,
                rules: settings.rules,
                break_duration: settings.break_duration,
                prev_alarm: None,
            }
        }

        pub fn run(mut self) {
            let am = AlarmManager::new(
                self.am_tx.clone(),
                self.am_tx.subscribe(),
                &self.rules,
                self.break_duration,
            );

            // Start AlarmManager inside Tauri runtime
            tauri::async_runtime::spawn(async move {
                am.run();
            });

            // UI + Alarm event loop
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::select! {
                        message1 = self.ui_rx.recv() => {
                            if let Some(msg) = message1 {
                                self.handle_ui_message(msg);
                            }
                        }
                        message2 = self.am_rx.recv() => {
                            if let Ok(msg) = message2 {
                                self.handle_am_message(msg);
                            }
                        }
                    }
                }
            });
        }

        fn handle_am_message(&mut self, msg: Message) {
            match msg.typ {
                MessageType::EvtNextAlarm => self.handle_next_alarm(msg.payload),
                MessageType::EvtPlayingAlarm => self.handle_playing_alarm(msg.payload),
                _ => (),
            }
        }

        fn handle_next_alarm(&self, payload: Payload) {
            let json: Value = match payload {
                Payload::Alarm(alarm) => match alarm {
                    Some(alarm) => json!({
                        "next-alarm": alarm,
                        "prev-alarm": self.prev_alarm
                    }),
                    None => json!({
                        "next-alarm": null,
                        "prev-alarm": null
                    }),
                },
                _ => return,
            };

            self.win_handle
                .emit(&MessageType::EvtNextAlarm.to_string(), json.to_string())
                .unwrap();
        }

        fn handle_playing_alarm(&mut self, payload: Payload) {
            if let Payload::Alarm(Some(alarm)) = payload {
                self.prev_alarm = Some(alarm);
            }
        }

        fn handle_ui_message(&mut self, msg: String) {
            let json = match serde_json::from_str::<serde_json::Value>(&msg) {
                Ok(parsed) => parsed,
                Err(_) => return,
            };

            if let Some(typ) = json.get("type").and_then(|n| n.as_str()) {
                match MessageType::from_str(typ) {
                    Some(MessageType::CmdUpdateRules) => self.handle_update_rules(json),
                    Some(MessageType::CmdUpdateBreakDuration) => {
                        self.handle_update_break_duration(json)
                    }
                    Some(MessageType::CmdStartup) => self.handle_startup(),
                    _ => (),
                }
            }
        }

        fn handle_startup(&self) {
            let c = Message {
                typ: MessageType::CmdUpdateAlarms,
                payload: Payload::Rules(self.rules.clone()),
            };

            self.am_tx.send(c).unwrap();
            self.am_tx
                .send(Message {
                    typ: MessageType::CmdUpdateBreakDuration,
                    payload: Payload::BreakDuration(self.break_duration),
                })
                .unwrap();

            let json = json!({
                "rules": serde_json::to_string(&self.rules).unwrap(),
                "break_duration": self.break_duration
            });

            self.win_handle
                .emit(&MessageType::EvtStarted.to_string(), json.to_string())
                .unwrap();
        }

        fn handle_update_rules(&mut self, json: serde_json::Value) {
            let mut rule_objects: Vec<Rule> = Vec::new();

            if let Some(rules) = json.get("rules").and_then(serde_json::Value::as_array) {
                for rule_json in rules {
                    let rule: Rule = serde_json::from_value(rule_json.clone())
                        .expect("Rule deserialization error");
                    rule_objects.push(rule);
                }
            }

            if let Err(error) = self.save_settings(&rule_objects, self.break_duration) {
                log::error!("Could not save rules: {}", error);
                return;
            }

            let c = Message {
                typ: MessageType::CmdUpdateAlarms,
                payload: Payload::Rules(rule_objects.clone()),
            };

            self.am_tx.send(c).unwrap();
            self.rules = rule_objects;

            let json = json!({
                "rules": serde_json::to_string(&self.rules).unwrap(),
                "break_duration": self.break_duration
            });

            self.win_handle
                .emit(&MessageType::EvtRulesApplied.to_string(), json.to_string())
                .unwrap();
        }

        fn handle_update_break_duration(&mut self, json: Value) {
            let Some(duration) = json
                .get("break_duration")
                .and_then(Value::as_u64)
                .filter(|n| (1..=5).contains(n))
            else {
                return;
            };
            if let Err(error) = self.save_settings(&self.rules, duration) {
                log::error!("Could not save break duration: {}", error);
                let _ = self.win_handle.emit(
                    &MessageType::EvtBreakDurationApplied.to_string(),
                    json!({"failed": true}).to_string(),
                );
                return;
            }
            self.break_duration = duration;
            self.am_tx
                .send(Message {
                    typ: MessageType::CmdUpdateBreakDuration,
                    payload: Payload::BreakDuration(duration),
                })
                .unwrap();
            self.win_handle
                .emit(
                    &MessageType::EvtBreakDurationApplied.to_string(),
                    json!({"break_duration": duration}).to_string(),
                )
                .unwrap();
        }

        fn save_settings(&self, rules: &[Rule], break_duration: u64) -> std::io::Result<()> {
            let settings = Settings {
                rules: rules.to_vec(),
                break_duration,
            };
            let serialized = serde_json::to_string(&settings).unwrap();
            let path = std::path::PathBuf::from(get_settings_file_name());
            let temporary = path.with_extension("json.tmp");
            std::fs::write(&temporary, serialized)?;
            std::fs::rename(temporary, path)
        }

        fn read_settings() -> Settings {
            let path = get_settings_file_name();

            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(&path).parent() {
                std::fs::create_dir_all(parent).unwrap();
            }

            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .unwrap();

            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();

            Settings::parse(&contents)
        }
    }
}

pub use ui_handler::*;
