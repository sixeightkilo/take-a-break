# Take a break!
This app reminds you to take small breaks while working on desktop/laptop.
Supports Linux and macOS.

# Building
From the src-tauri directory:

$cargo install tauri-cli

$cargo tauri build

To include the 1-minute interval for testing, build with the `debug` feature:

`cargo tauri build --features debug`

# Screenshot

![Screenshot 2023-12-06 at 8 16 18 PM](https://github.com/kargirwar/take-a-break/assets/33156462/4317b149-4d8a-49d4-8c97-0fcd9ac4ad6b)


# Break duration
Choose a global break duration of 1–5 minutes above the schedule rules. Changes
save automatically and apply from the next break start; a break already underway
keeps its original duration. Existing settings default to one minute.

The existing beep pattern marks the start, and four short, higher-pitched beeps
mark the end. Break-end reminders do not change scheduled start times. Sounds
are suppressed while the desktop is locked, and stale end reminders are discarded
after a prolonged sleep. Closing the app cancels pending end reminders.
