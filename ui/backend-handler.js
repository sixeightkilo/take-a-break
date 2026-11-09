import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Constants } from './constants.js'
import { PubSub } from './pubsub.js'
import { Utils } from './utils.js'
import { Logger } from './logger.js'

const TAG = "backend-handler";
// const appWindow = window.__TAURI__.window.appWindow;
// const { invoke } = window.__TAURI__.tauri;
const appWindow = getCurrentWindow();


class BackendHandler {
    constructor() {
        this.ready = this.init();
    }

    async init() {
        PubSub.subscribe(Constants.EVENT_RULES_UPDATED, (e) => {
            Logger.Log(TAG, JSON.stringify(e.rules));
            invoke('command', {
                "payload": JSON.stringify({
                    type: Constants.CMD_UPDATE_RULES,
                    rules: e.rules})
            }).then((response) => {
                Logger.Log(TAG, response);
            });
        });

        PubSub.subscribe(Constants.EVENT_BREAK_DURATION_UPDATED, (e) => {
            invoke('command', {
                payload: JSON.stringify({type: Constants.CMD_UPDATE_BREAK_DURATION, break_duration: e.break_duration})
            }).catch(() => PubSub.publish(Constants.EVENT_BREAK_DURATION_APPLIED, {failed: true}));
        });

        await appWindow.listen(Constants.EVENT_BREAK_DURATION_APPLIED, (e) => {
            PubSub.publish(Constants.EVENT_BREAK_DURATION_APPLIED, JSON.parse(e.payload));
        });

        PubSub.subscribe(Constants.EVENT_DOM_LOADED, (e) => {
            invoke('command', {
                "payload": JSON.stringify({ type: Constants.CMD_STARTUP })
            }).then((response) => {
                Logger.Log(TAG, response);
            });
        });

        await appWindow.listen(Constants.EVENT_NEXT_ALARM, (e) => {
            Logger.Log(TAG, e.payload);

            PubSub.publish(Constants.EVENT_NEXT_ALARM, {
                alarms: JSON.parse(e.payload)
            });
        });

        await appWindow.listen(Constants.EVENT_RULES_APPLIED, (e) => {
            let json = JSON.parse(e.payload);
            Logger.Log(TAG, `EVENT_RULES_APPLIED: ${json.rules}`);

            PubSub.publish(Constants.EVENT_RULES_APPLIED, {
                rules: JSON.parse(json.rules),
                break_duration: json.break_duration
            });
        });

        await appWindow.listen(Constants.EVENT_STARTED, (e) => {
            let json = JSON.parse(e.payload);
            Logger.Log(TAG, `EVENT_STARTED: ${json.rules}`);

            PubSub.publish(Constants.EVENT_STARTED, {
                rules: JSON.parse(json.rules),
                break_duration: json.break_duration
            });
        });

        Logger.Log(TAG, "Listening to events");
    }
}

export { BackendHandler }
