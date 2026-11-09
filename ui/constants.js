class Constants {
    static get PAGE_CHANGE() {
        return 'page-change';
    }

    static get PAGE_RULE_EDITOR() {
        return 'page-rule-editor';
    }

    static get PAGE_INDEX() {
        return 'page-index';
    }

    static get PAGE_RULES() {
        return 'page-rules';
    }

    static get PAGE_HELP() {
        return 'page-help';
    }

    //js events
    static get EVENT_DOM_LOADED() {
        return 'event-dom-loaded';
    }

    static get EVENT_RULES_UPDATED() {
        return 'event-rules-updated';
    }

    static get EVENT_NEXT_ALARM() {
        return 'event-next-alarm';
    }

    static get EVENT_NEW_RULE() {
        return 'event-next-rule';
    }

    static get EVENT_BREAK_DURATION_UPDATED() { return 'event-break-duration-updated'; }
    static get CMD_UPDATE_BREAK_DURATION() { return 'cmd-update-break-duration'; }
    static get EVENT_BREAK_DURATION_APPLIED() { return 'event-break-duration-applied'; }

    //for rust
    static get CMD_UPDATE_RULES() {
        return 'cmd-update-rules';
    }

    static get CMD_STARTUP() {
        return 'cmd-startup';
    }

    //from rust
    static get EVENT_STARTED() {
        return 'event-started';
    }

    static get EVENT_RULES_APPLIED() {
        return 'event-rules-applied';
    }
}

export { Constants }
