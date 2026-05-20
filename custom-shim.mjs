import { WorkerEntrypoint } from "cloudflare:workers";
import defaultModule from "./index_bg.wasm";
import __wbg_init, { fetch, setPanicHook, __wbg_reset_state } from "./index.js";

await __wbg_init(defaultModule);

let criticalError = false;

const registerPanicHook = () => setPanicHook(message => {
  const panicError = new Error(`Rust panic: ${message}`);
  console.error('Critical', panicError);

  criticalError = true;
});

registerPanicHook();

let instanceId = 0;
const checkReinitialise = () => {
  if (criticalError) {
    console.log("Reinitialising Wasm application (due to critical error that occurred earlier)");
    console.info(`Instance ID is ${instanceId}.`)

    __wbg_reset_state();
    registerPanicHook();

    criticalError = false;
    instanceId++;
  }
}

addEventListener('error', e => handleMaybeCritical(e.error));

const handleMaybeCritical = e => {
  if (e instanceof WebAssembly.RuntimeError) {
    console.error('Critical', e);
    criticalError = true;
  } else {
    console.error('Error', e);
  }
}

class Entrypoint extends WorkerEntrypoint {
  constructor(ctx, env) {
    super(ctx, env);

    this.ctx = ctx;
    this.env = env;
  }

  async fetch(req) {
    return await fetch(req, this.env, this.ctx);
  }
}

// Proxy hooks which are "transparent" to make sure that the object is still an Object
const instanceProxyHooks = {
  set: (target, prop, value, receiver) => Reflect.set(target.instance, prop, value, receiver),
  has: (target, prop) => Reflect.has(target.instance, prop),
  deleteProperty: (target, prop) => Reflect.deleteProperty(target.instance, prop),
  apply: (target, thisArg, args) => Reflect.apply(target.instance, thisArg, args),
  construct: (target, args, newTarget) => Reflect.construct(target.instance, args, newTarget),
  getPrototypeOf: (target) => Reflect.getPrototypeOf(target.instance),
  setPrototypeOf: (target, proto) => Reflect.setPrototypeOf(target.instance, proto),
  isExtensible: (target) => Reflect.isExtensible(target.instance),
  preventExtensions: (target) => Reflect.preventExtensions(target.instance),
  getOwnPropertyDescriptor: (target, prop) => Reflect.getOwnPropertyDescriptor(target.instance, prop),
  defineProperty: (target, prop, descriptor) => Reflect.defineProperty(target.instance, prop, descriptor),
  ownKeys: (target) => Reflect.ownKeys(target.instance),
};

const classProxyHooks = {
  construct(ctor, args, newTarget) {
    try {
      checkReinitialise();

      const instance = {
        instance: Reflect.construct(ctor, args, newTarget),
        instanceId,
        ctor,
        args,
        newTarget
      };

      return new Proxy(instance, {
        ...instanceProxyHooks,
        get(target, prop, receiver) {
          if (target.instanceId !== instanceId) {
            target.instance = Reflect.construct(target.ctor, target.args, target.newTarget);
            target.instanceId = instanceId;
          }

          const original = Reflect.get(target.instance, prop, receiver);
          if (typeof original !== 'function') return original;

          if (original.constructor === Function) {
            return new Proxy(original, {
              apply(target, thisArg, argArray) {
                checkReinitialise();

                try {
                  return target.apply(thisArg, argArray);
                } catch (e) {
                  handleMaybeCritical(e);
                  throw e;
                }
              }
            });
          } else {
            return new Proxy(original, {
              async apply(target, thisArg, argArray) {
                checkReinitialise();

                try {
                  return await target.apply(thisArg, argArray);
                } catch (e) {
                  handleMaybeCritical(e);
                  throw e;
                }
              }
            });
          }
        }
      });
    } catch (e) {
      criticalError = true;
      throw e;
    }
  }
};

export default new Proxy(Entrypoint, classProxyHooks);