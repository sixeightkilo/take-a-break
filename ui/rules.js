import { Constants } from './constants.js'
import { PubSub } from './pubsub.js'
import { Utils } from './utils.js'
import { Logger } from './logger.js'

const TAG = "rules";

class Rules {
    constructor($root) {
        this.$root = $root;
        this.rootTemplate = document.getElementById('rules-template').innerHTML;
        this.ruleTemplate = document.getElementById('rule-template').innerHTML;
        this.rules = [];
        this.bindHandlers();
    }

    bindHandlers() {
        this.$root.addEventListener('click', (e) => {
            let $n = e.target;
            if ($n.classList.contains('del-rule')) {
                $n.closest('.rule').remove();

                //just re-read everything so we don't have to worry about serial
                this.rules = this.getRules();
                //this is necessary so that dom is in step with the object in memory
                this.updateSerial();

                PubSub.publish(Constants.EVENT_RULES_UPDATED, {
                    rules: this.getRules(true)
                });

                if (this.rules.length == 0) {
                    this.showError(true);
                }
            }
        });

        this.$root.addEventListener('click', (e) => {
            let $n = e.target;
            if ($n.classList.contains('save-rule')) {
                if (!this.isValid($n.parentElement.parentElement, parseInt(e.target.dataset.serial))) {
                    return;
                }

                this.disableRule($n.parentElement);

                this.rules = this.getRules();
                this.updateSerial();

                PubSub.publish(Constants.EVENT_RULES_UPDATED, {
                    rules: this.getRules(true)
                });

                Utils.info("Saved", 2000);
            }
        });

        //Track changes
        this.$root.addEventListener('change', (e) => {
            let $p = e.target.closest('.rule');
            $p.querySelector('.save-rule').style.display = 'block';
        });

        PubSub.subscribe(Constants.EVENT_NEW_RULE, () => {
            Logger.Log(TAG, "EVENT_NEW_RULE");
            this.showError(false);
            this.addRule();
            this.updateSerial();
        });
    }

    load(rules) {
        this.$root.replaceChildren(Utils.generateNode(this.rootTemplate, {}));
        this.$message = this.$root.querySelector('.no-rules-message');
        this.$title = this.$root.querySelector('.rule-title');

        /*fix list height for scrolling*/
        this.$list = this.$root.querySelector('#rules-content');
        this.$list.style.height = '100%';

        this.loadRules(rules);
    }

    loadRules(rules) {
        Logger.Log(TAG, "loadRules:" + rules);
        if (rules.length == 0) {
            this.showError(true);
            return;
        }

        this.showError(false);

        for (let i = 0; i < rules.length; i++) {
            this.addRule(rules[i]);
        }

        this.rules = rules;
        this.updateSerial();
    }

    //TODO: need better name for this
    showError(show) {
        if (show) {
            this.$message.style.display = 'block';
            this.$title.style.display = 'none';
        } else {
            this.$message.style.display = 'none';
            this.$title.style.display = 'grid';
        }
    }

    addRule(rule = {}) {
        let $n = Utils.generateNode(this.ruleTemplate, {});
        this.$list.append($n);

        if (Utils.isEmpty(rule)) {
            return;
        }

        $n = this.$root.querySelectorAll('.rule:last-child')[0];
        $n.querySelector('.interval').querySelectorAll('option').forEach((e) => {
            if (parseInt(e.value) == rule.interval) {
                e.selected = true;
                e.defaultSelected = true;
            } else {
                e.selected = false;
                e.defaultSelected = false;
            }
        });

        $n.querySelectorAll('input[name="days"]').forEach((e) => {
            if (rule.days.includes(e.value)) {
                e.checked = true;
            } else {
                e.checked = false;
            }
        });

        $n.querySelector('.from').querySelectorAll('option').forEach((e) => {
            if (parseInt(e.value) == rule.from) {
                e.selected = true;
                e.defaultSelected = true;
            } else {
                e.selected = false;
                e.defaultSelected = false;
            }
        });

        $n.querySelector('.to').querySelectorAll('option').forEach((e) => {
            if (parseInt(e.value) == rule.to) {
                e.selected = true;
                e.defaultSelected = true;
            } else {
                e.selected = false;
                e.defaultSelected = false;
            }
        });

        //this is a saved rule. Start disabled
        this.disableRule($n);
    }

    updateSerial() {
        let serial = 1;
        [...document.querySelectorAll('.save-rule')].forEach((e) => {
            e.dataset.serial = serial++;
        });
    }

    isValid($r, serial = null) {
        let from = parseInt($r.querySelector('.from').value);
        let to = parseInt($r.querySelector('.to').value);

        let days = [];
        $r.querySelectorAll('input[name="days"]').forEach((r) => {
            if (r.checked) {
                days.push(r.value);
            }
        });

        Logger.Log(TAG, days);

        if (days.length == 0) {
            Utils.alert("Please select days", 3000);
            return false;
        }

        if (to <= from) {
            Utils.alert("To hours must be greater than from hours", 3000);
            return false;
        }

        if (this.isDuplicate(this.getRule($r).rule, serial)) {
            Utils.alert("Duplicate rule", 3000);
            return false;
        }

        Logger.Log(TAG, JSON.stringify(this.getRules()));

        return true;
    }

    disableRule($r) {
        $r.style.borderColor = 'grey';
        $r.querySelector('.save-rule').style.display = 'none';
    }

    isDuplicate(r, serial = null) {
        //for any given day there can be only rule for a set of from-to
        for (let i = 0; i < this.rules.length; i++) {
            let o = this.rules[i];
            if (serial === o.serial) {
                //don't compare with self
                continue;
            }
            //let commonDays = _.intersection(r.days, o.days);
            let commonDays = r.days.filter(x => o.days.includes(x));
            if (commonDays.length == 0) {
                continue;
            }

            let range1 = Utils.range(r.from, r.to);
            let range2 = Utils.range(o.from, o.to);

            let commonHours = range1.filter(x => range2.includes(x));
            Logger.Log(TAG, `commonHours: ${commonHours}`);

            if (commonHours.length > 1) {
                return true;
            }
        }

        return false;
    }

    getRules(onlySaved = false) {
        let rules = [];
        let serial = 1;
        let $rules = document.querySelectorAll('.rule');

        for (let i = 0; i < $rules.length; i++) {
            let r = {};
            r.serial = serial;
            let result = this.getRule($rules[i]);

            if (onlySaved == true && result.isSaved == false) {
                continue;
            }

            Object.assign(r, result.rule);
            rules.push(r);
            serial++;
        }

        return rules;
    }

    getRule($r) {
        let r = {};
        r.days = [];

        [...$r.querySelectorAll('[name=days]')].forEach(($d) => {
            if ($d.checked) {
                r.days.push($d.value);
            }
        });

        r.interval = parseInt($r.querySelector('.interval').value);
        r.from = parseInt($r.querySelector('.from').value);
        r.to = parseInt($r.querySelector('.to').value);

        let isSaved = ($r.querySelector('.save-rule').style.display == 'none') ? true : false;

        return {rule: r, isSaved: isSaved};
    }
}

export { Rules }
