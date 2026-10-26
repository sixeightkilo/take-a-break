import { invoke } from "@tauri-apps/api/core";
import { Constants } from './constants.js'
import { PubSub } from './pubsub.js'
import { Utils } from './utils.js'
import { Logger } from './logger.js'
import { App } from './app.js'
import { Help } from './help.js'
//pages
import { BackendHandler } from './backend-handler.js'

const TAG = "router";

class Router {
    constructor() {
        document.addEventListener('DOMContentLoaded', async () => {
            await this.init();
            Logger.Log(TAG, "DOMContentLoaded");
            PubSub.publish(Constants.EVENT_DOM_LOADED, {});
        }) 
    }

    async init() {
        if (await invoke('is_debug_build')) {
            const ruleTemplate = document.getElementById('rule-template');
            ruleTemplate.innerHTML = ruleTemplate.innerHTML.replace(
                '<option value="10">10 minutes</option>',
                '<option value="1">1 minute</option><option value="10">10 minutes</option>'
            );
        }

        this.$container = document.getElementById('container');

        this.app = new App(this.$container);
        this.app.load();

        this.help = new Help(this.$container);

        PubSub.subscribe(Constants.PAGE_HELP, (e) => {
            this.help.load();
        });

        PubSub.subscribe(Constants.PAGE_APP, (e) => {
            this.app.load();
        });

        new BackendHandler();
    }
}

new Router()
