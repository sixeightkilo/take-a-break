//! Keeps scheduled break starts on wall-clock times and checks independent
//! break-end deadlines each second. Sound playback runs off the async scheduler.

mod alarm_utils;
mod alarm_manager {

    use super::alarm_utils::*;
    use crate::player::{play, play_end};
    use crate::utils::{is_locked, LockedState};
    // use crate::Alarm;
    // use crate::{BcastReceiver, BcastSender, Message, MessageType, Payload};
    use crate::ui_handler::{
        Alarm, BcastReceiver, BcastSender, Message, MessageType, Payload, Rule,
    };

    use chrono::{offset::Local, Datelike, Timelike, Weekday};
    use std::time::Duration;

    use log::debug;
    use std::collections::HashMap;

    struct BreakReminders {
        duration: u64,
        deadlines: Vec<i64>,
    }

    impl Default for BreakReminders {
        fn default() -> Self {
            Self {
                duration: 1,
                deadlines: Vec::new(),
            }
        }
    }

    impl BreakReminders {
        fn start(&mut self, now_ms: i64) {
            self.deadlines.push(now_ms + self.duration as i64 * 60_000);
        }

        fn take_due(&mut self, now_ms: i64) -> usize {
            let mut due = 0;
            self.deadlines.retain(|deadline| {
                if *deadline > now_ms {
                    return true;
                }
                // Discard old reminders after a prolonged sleep rather than replaying them.
                if now_ms - *deadline < 60_000 {
                    due += 1;
                }
                false
            });
            due
        }
    }

    pub struct AlarmManager {
        tx: BcastSender<Message>,
        rx: BcastReceiver<Message>,
        alarms: HashMap<Weekday, HashMap<usize, Vec<usize>>>,
        reminders: BreakReminders,
        last_minute: Option<i64>,
    }

    impl AlarmManager {
        pub fn new(
            tx: BcastSender<Message>,
            rx: BcastReceiver<Message>,
            rules: &[Rule],
            duration: u64,
        ) -> Self {
            let alarms = get_alarms(rules);
            Self {
                tx,
                rx,
                alarms,
                reminders: BreakReminders {
                    duration,
                    ..BreakReminders::default()
                },
                last_minute: None,
            }
        }

        pub fn run(mut self) {
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(1));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        msg = self.rx.recv() => {
                            match msg {
                                Ok(i) => self.handle_message(i),
                                Err(e) => debug!("{}", e),
                            }
                        }

                        _ = ticker.tick() => {
                            self.handle_timer_expiry();
                        }
                    }
                }
            });
        }

        /// Check end deadlines each second and scheduled starts once per wall-clock minute.
        fn handle_timer_expiry(&mut self) {
            let now = Local::now();
            if self.reminders.take_due(now.timestamp_millis()) > 0 {
                tauri::async_runtime::spawn_blocking(play_end);
            }
            let minute = now.timestamp().div_euclid(60);
            if self.last_minute == Some(minute) {
                return;
            }
            self.last_minute = Some(minute);
            let current_weekday: Weekday = now.weekday();
            let current_hour: usize = now.hour() as usize;
            let current_minute: usize = now.minute() as usize;

            if let Some(hour_map) = self.alarms.get(&current_weekday) {
                if let Some(minute_vec) = hour_map.get(&current_hour) {
                    if minute_vec.contains(&current_minute) {
                        debug!("alarm_manager: playing alarm");
                        if !matches!(is_locked(), LockedState::Locked) {
                            self.reminders.start(now.timestamp_millis());
                            tauri::async_runtime::spawn_blocking(play);
                        }

                        let c = Message {
                            typ: MessageType::EvtPlayingAlarm,
                            payload: Payload::Alarm(Some(Alarm {
                                day: current_weekday,
                                hour: current_hour,
                                min: current_minute,
                            })),
                        };

                        self.tx.send(c).unwrap();
                    }
                }
            }

            //notifying next alarm every minute helps to minimize
            //the time wrong value is displayed after waking up from sleep
            self.notify_next_alarm();
        }

        /// Handles message from ui_handlers
        fn handle_message(&mut self, msg: Message) {
            debug!("alarm_manager:{:?}", msg);
            match msg.typ {
                MessageType::CmdUpdateBreakDuration => {
                    if let Payload::BreakDuration(duration) = msg.payload {
                        self.reminders.duration = duration;
                    }
                }
                MessageType::CmdUpdateAlarms => {
                    self.update_alarms(msg.payload);
                    self.notify_next_alarm();
                }
                _ => debug!("alarm_manager::Unknown command"),
            }
        }

        fn notify_next_alarm(&self) {
            let now = Local::now();
            let current_weekday: Weekday = now.weekday();
            let current_hour: usize = now.hour() as usize;
            let current_minute: usize = now.minute() as usize;

            let c = Message {
                typ: MessageType::EvtNextAlarm,
                payload: Payload::Alarm(find_next_alarm(
                    &self.alarms,
                    current_weekday,
                    current_hour,
                    current_minute,
                )),
            };

            self.tx.send(c).unwrap();
        }

        fn update_alarms(&mut self, payload: Payload) {
            let rules;
            if let Payload::Rules(temp) = payload {
                rules = temp;
            } else {
                return;
            }

            self.alarms = get_alarms(&rules);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ui_handler::Rule;
        // Keep the existing schedule assertions without an extra dependency.
        macro_rules! hashmap { ($($key:expr => $value:expr),* $(,)?) => { HashMap::from([$(($key, $value)),*]) }; }

        #[test]
        fn duration_changes_only_affect_future_breaks() {
            let mut reminders = BreakReminders::default();
            reminders.duration = 3;
            reminders.start(0);
            reminders.duration = 5;
            assert_eq!(reminders.take_due(179_999), 0);
            assert_eq!(reminders.take_due(180_000), 1);
            assert_eq!(reminders.take_due(180_001), 0);
            reminders.start(1_800_000);
            assert_eq!(reminders.take_due(2_099_999), 0);
            assert_eq!(reminders.take_due(2_100_000), 1);
        }

        #[test]
        fn handles_overlapping_breaks_and_discards_stale_reminders() {
            let mut reminders = BreakReminders::default();
            reminders.duration = 3;
            reminders.start(0);
            reminders.start(60_000);
            assert_eq!(reminders.take_due(180_000), 1);
            assert_eq!(reminders.take_due(240_000), 1);
            reminders.start(300_000);
            assert_eq!(reminders.take_due(600_000), 0);
            assert!(reminders.deadlines.is_empty());
        }

        #[test]
        fn test_get_alarms() {
            let rule1 = Rule {
                serial: 1,
                days: vec!["Tue".to_string(), "Wed".to_string()],
                interval: 1,
                from: 18,
                to: 19,
            };

            let rule2 = Rule {
                serial: 2,
                days: vec!["Tue".to_string()],
                interval: 30,
                from: 19,
                to: 20,
            };

            let rules = vec![rule1, rule2];
            let alarms = get_alarms(&rules);
            debug!("{:?}", alarms);

            let mins: Vec<usize> = (1..=59).collect();
            let expected = hashmap! {
                Weekday::Tue => hashmap! {
                    18 => mins.clone(),
                    19 => vec![0, 30],
                    20 => vec![0],
                },
                Weekday::Wed => hashmap! {
                    18 => mins.clone(),
                    19 => vec![0],
                },
            };

            assert!(alarms.contains_key(&Weekday::Tue));
            assert!(alarms.contains_key(&Weekday::Wed));
            assert_eq!(alarms, expected);
        }

        #[test]
        fn test_next_1() {
            let rule1 = Rule {
                serial: 1,
                days: vec!["Fri".to_string()],
                interval: 30,
                from: 17,
                to: 18,
            };

            let rule2 = Rule {
                serial: 2,
                days: vec!["Sun".to_string()],
                interval: 30,
                from: 19,
                to: 20,
            };

            let rules = vec![rule1, rule2];
            let alarms = get_alarms(&rules);

            let expected = hashmap! {
                Weekday::Fri => hashmap! {
                    17 => vec![30],
                    18 => vec![0],
                },
                // Add more weekdays and alarms as needed
                Weekday::Sun => hashmap! {
                    19 => vec![30],
                    20 => vec![0],
                },
            };

            assert_eq!(alarms, expected);

            assert_eq!(
                find_next_alarm(&alarms, Weekday::Fri, 17, 58),
                Some(Alarm {
                    day: Weekday::Fri,
                    hour: 18,
                    min: 0
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sat, 18, 0),
                Some(Alarm {
                    day: Weekday::Sun,
                    hour: 19,
                    min: 30
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sun, 18, 0),
                Some(Alarm {
                    day: Weekday::Sun,
                    hour: 19,
                    min: 30
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sun, 19, 0),
                Some(Alarm {
                    day: Weekday::Sun,
                    hour: 19,
                    min: 30
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sun, 19, 20),
                Some(Alarm {
                    day: Weekday::Sun,
                    hour: 19,
                    min: 30
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sun, 19, 31),
                Some(Alarm {
                    day: Weekday::Sun,
                    hour: 20,
                    min: 0
                })
            );
            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sun, 20, 31),
                Some(Alarm {
                    day: Weekday::Fri,
                    hour: 17,
                    min: 30
                })
            );
        }

        #[test]
        fn test_next_2() {
            let rule1 = Rule {
                serial: 1,
                days: vec!["Sat".to_string()],
                interval: 2,
                from: 12,
                to: 13,
            };

            let rules = vec![rule1];
            let alarms = get_alarms(&rules);

            let mins: Vec<usize> = (2..=58).step_by(2).collect();
            let expected = hashmap! {
                Weekday::Sat => hashmap! {
                    12 => mins,
                    13 => vec![0],
                },
            };

            assert_eq!(alarms, expected);

            assert_eq!(
                find_next_alarm(&alarms, Weekday::Sat, 13, 12),
                Some(Alarm {
                    day: Weekday::Sat,
                    hour: 12,
                    min: 2
                })
            );
        }
    }
}

pub use alarm_manager::*;
