//import { jwtDecode, InvalidTokenError, JwtPayload } from "./mods/jwt-decode.js";

type ActionTask = (element: HTMLElement, args: string[]) => void

type ActionTrigger = {
    namespace?: string,
    listeners: Action[]
}

type PaneContexts = "signup" | "login" | "loading"

type Action = {
    task: ActionTask,
    element: HTMLElement,
    arguments: string[],
    trigger: ActionTrigger
}

const actions: Record<string, ActionTask> = {
    "switch-to": (_, args) => {
        PaneContext.switchTo(args[0] as PaneContexts, ...args.slice(1))
    },
}

const triggers: Record<string, ActionTrigger> = {
    "click": {
        listeners: []
    },

    "async:load-done": {
        namespace: "async",
        listeners: []
    }
}

const fireEvent = (trigger: ActionTrigger, predicate: (action: Action) => boolean) => {
    for (const action of trigger.listeners.filter(predicate)) {
        action.task.call(action.element, action.element, action.arguments)
    }
}

const contexts: Record<string, string> = {
    "signup": "./signup.pane.html",
    "login": "./login.pane.html",
    "loading": "./loading.pane.html"
}

const PaneContext = {
    async getPane(name: string) {
        return await (await fetch(contexts[name])).text()
    },

    fillTemplate(template: string, args: string[]): string {
        return template.replaceAll(/\[\$(\d+)]/g, (_, p) => {
            console.log(p[0])

            return args[Number(p[0])]
        })
    },

    async switchTo(ctx: PaneContexts | string, ...args: any[]) {
        console.log(ctx, args)

        const pane = document.getElementById("pane");

        const template = contexts[ctx] ? await PaneContext.getPane(ctx) : ctx;

        pane!.innerHTML = PaneContext.fillTemplate(template, args)

        for (const script of document.querySelectorAll("#pane script[data-setup]")) {
            eval(`{ (async () => { ${script.innerHTML} })() }`)
        }

        for (const el of document.querySelectorAll("#pane [data-trigger]")) {
            if ((el as HTMLElement).dataset.action && (el as HTMLElement).dataset.trigger) {
                const action: Action = {
                    task: actions[(el as HTMLElement).dataset.action!.split(":")[0]],
                    element: el as HTMLElement,
                    arguments: (el as HTMLElement).dataset.action!.split(":").length > 1 ? (el as HTMLElement).dataset.action!.split(":")[1].split(",") : [],
                    trigger: triggers[(el as HTMLElement).dataset.trigger!],
                } satisfies Action;

                triggers[(el as HTMLElement).dataset.trigger!].listeners.push(action)

                if ((el as HTMLElement).dataset.trigger! === "click") {
                    action.element.addEventListener("click", _ => {
                        fireEvent(action.trigger, a => a.element === action.element)
                    })
                }
            }
        }

        for (const script of document.querySelectorAll("#pane script")) {
            eval(`{ (async () => { ${script.innerHTML} })() }`)
        }
    }
}

// @ts-ignore
window.paneContext = {
    registerAction(name: string, action: ActionTask) {
        actions[name] = action
    },

    registerTrigger(namespace: string, name: string) {
        triggers[namespace + ":" + name] = {
            namespace,
            listeners: []
        }
    },

    fireTrigger: fireEvent,

    switchTo: PaneContext.switchTo,
    getPane: PaneContext.getPane,

    fillTemplate: PaneContext.fillTemplate,
}

PaneContext.switchTo("loading", ["login"])