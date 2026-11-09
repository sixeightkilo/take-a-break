import { Constants } from './constants.js'
import { PubSub } from './pubsub.js'
import { Utils } from './utils.js'
import { Logger } from './logger.js'
import { Tabs } from './tabs.js'
import { Rules } from './rules.js'

const TAG = "app";

class App {
    constructor($root) {
        this.$root = $root;
        this.rootTemplate = document.getElementById('app-template').innerHTML;

        this.$root.replaceChildren(Utils.generateNode(this.rootTemplate, {}));
        this.rules = [];
        this.breakDuration = 1;
        this.settingsLoaded = false;
        this.bindHandlers();
    }

    bindHandlers() {
        //this event is raised when backend applies already saved rules
        PubSub.subscribe(Constants.EVENT_STARTED, (e) => {
            Logger.Log(TAG, JSON.stringify(e));
            this.rules = e.rules;
            this.breakDuration = e.break_duration;
            this.settingsLoaded = true;
            this.updateBreakDuration();
            let rules = new Rules(this.$root.querySelector('.tab-content.rules'));
            rules.load(this.rules);
            this.$numOfRules.innerHTML = this.rules.length;
        });

        //this event is raised when front end changes any rules
        PubSub.subscribe(Constants.EVENT_RULES_APPLIED, (e) => {
            Logger.Log(TAG, JSON.stringify(e));
            this.rules = e.rules;
            this.breakDuration = e.break_duration;
            this.settingsLoaded = true;
            this.updateBreakDuration();
            this.$numOfRules.innerHTML = this.rules.length;
        });

        PubSub.subscribe(Constants.EVENT_NEXT_ALARM, (e) => {
            Logger.Log(TAG, JSON.stringify(e.alarms));
            this.next = e.alarms['next-alarm'] ?? null;
            this.prev = e.alarms['prev-alarm'] ?? null;
            this.updateTitleBar();
        });

        PubSub.subscribe(Constants.EVENT_BREAK_DURATION_APPLIED, (e) => {
            if (!e.failed) this.breakDuration = e.break_duration;
            this.updateBreakDuration();
            if (e.failed) Utils.alert("Could not save break duration", 3000);
            else Utils.info("Saved. Applies from the next break start.", 2000);
        });

        this.$root.addEventListener('change', (e) => {
            if (e.target.id !== 'break-duration') return;
            const duration = Number(e.target.value);
            if (!Number.isInteger(duration) || duration < 1 || duration > 5) return;
            e.target.disabled = true;
            PubSub.publish(Constants.EVENT_BREAK_DURATION_UPDATED, {break_duration: duration});
        });

        this.$root.addEventListener('click', (e) => {
            if (e.target.classList.contains('add-rule')) {
                Logger.Log(TAG, "add rule");
                PubSub.publish(Constants.EVENT_NEW_RULE, {});
            }

            if (e.target.classList.contains('about')) {
                Logger.Log(TAG, "help");
                Logger.Log(TAG, "before help: " + JSON.stringify(this.rules));
                PubSub.publish(Constants.PAGE_HELP, {});
            }
        });
    }

    updateBreakDuration() {
        const select = this.$root.querySelector('#break-duration');
        if (!select) return;
        select.value = this.breakDuration;
        select.disabled = !this.settingsLoaded;
    }

    updateTitleBar() {
        this.$numOfRules.innerHTML = this.rules.length;

        if (this.next) {
            this.$nextAlarm.innerHTML = 
                `${this.next.day} ${Utils.zeroPad(this.next.hour)}:${Utils.zeroPad(this.next.min)}`;
        } else {
            this.$nextAlarm.innerHTML = "__";
        }

        if (this.prev) {
            this.$prevAlarm.innerHTML = 
                `${this.prev.day} ${Utils.zeroPad(this.prev.hour)}:${Utils.zeroPad(this.prev.min)}`;
            return;
        }

        this.$prevAlarm.innerHTML = "__";
    }

    load() {
        this.$root.replaceChildren(Utils.generateNode(this.rootTemplate, {}));

        this.$numOfRules = this.$root.querySelector('#num-of-rules');
        this.$nextAlarm = this.$root.querySelector('#next-alarm');
        this.$prevAlarm = this.$root.querySelector('#prev-alarm');

        let rules = new Rules(this.$root.querySelector('.tab-content.rules'));
        rules.load(this.rules);

        this.updateTitleBar();
        this.updateBreakDuration();
    }
}

export { App }
