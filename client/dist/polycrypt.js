"use strict";
const polycrypt = (() => {
    'use strict';
    //--------------------------------------------------------------------------
    if (window.crypto.subtle) {
        console.log("Polycrypt: Exposing browser implementation");
        return window.crypto.subtle;
    }
    else {
        console.log("Polycrypt: Using JavaScript polyfill implementation");
    }
    //--------------------------------------------------------------------------
    const userAgent = navigator.userAgent.toLowerCase();
    let browser = "";
    if (userAgent.indexOf('safari') != -1) {
        if (userAgent.indexOf('chrome') > -1) {
            browser = "Chrome";
        }
        else {
            browser = "Safari";
        }
    }
    else {
        browser = "Firefox";
    }
    if (browser == "Safari") {
        console.log("Safari might not support Uint8Arrays... check on that");
    }
    if (!navigator.cookieEnabled) {
        throw "PolyCrypt Dependency Error: Cookies are required to run Polycrypt.js";
    }
    try {
        localStorage["test"] = "Hello";
        const x = localStorage["test"];
        delete localStorage["test"];
    }
    catch (e) {
        throw "PolyCrypt Dependency Error: Access to localStorage required";
    }
    const host = window.location.href;
    if (host.indexOf(":") == -1 || host.indexOf("http") == -1) {
        throw "Polycrypt Host Error: You must access PolyCrypt from a localhost or the internet";
    }
    console.log("Compatibility check complete");
    //--------------------------------------------------------------------------
    let me = {
        polyfill: false
    }, // members of this object are appended to window.crypto
    nextOpId = 0, // a UID for each object this function instantiates
    // consts for dealing with the polycrypt backend
    backendSource = "http://polycrypt-test:8001/back/back.html", backendOrigin = "http://polycrypt-test:8001", backendFrame, backend = null, handleAlive;
    function loadBackendFrame() {
        backendFrame = document.createElement('iframe');
        backendFrame.src = backendSource;
        backendFrame.style.display = 'none';
        document.body.appendChild(backendFrame);
        backend = backendFrame.contentWindow;
        // @ts-ignore
        window.backendFrame = backendFrame;
    }
    window.addEventListener("load", loadBackendFrame);
    //--------------------------------------------------------------------------
    const PCMessage = {
        PC_MAGIC_COOKIE: "PolyCrypt-21A30E0E-1048-4ED0-BF7A-B1E01CA328E9",
        create: function PCMessage_create(partial) {
            const full = partial || {};
            full.cookie = this.PC_MAGIC_COOKIE;
            return full;
        },
        valid: function PCMessage_valid(msg) {
            return ((typeof (msg) === 'object') &&
                (msg.hasOwnProperty('cookie')) &&
                (msg.cookie === this.PC_MAGIC_COOKIE) &&
                (msg.hasOwnProperty('method') || msg.hasOwnProperty('event')));
        },
    };
    //--------------------------------------------------------------------------
    function Messenger(op, setResult) {
        // For event handling
        let listeners = {}, 
        // For back-end messaging
        myOpid = nextOpId++, myBackend = backend, myOrigin = backendOrigin, 
        // For KeyOperation / CryptoOperation
        _fireListener, _handleMessage;
        // define message handling functions
        this.addEventListener = function M_addEventListener(type, listener, useCapture) {
            console.log("TRACE Entered M_addEventListener");
            if (!(type in listeners)) {
                // @ts-ignore
                listeners[type] = [];
            }
            // @ts-ignore
            listeners[type].push(listener);
            // We ignore useCapture, because we're not in the DOM
        };
        this.removeEventListener = function M_removeEventListener(type, listener, useCapture) {
            console.log("TRACE Entered M_removeEventListener");
            // XXX: NOOP.  You can't remove listeners.
        };
        this.dispatchEvent = function M_dispatchEvent(e) {
            console.log("TRACE Entered M_dispatchEvent");
            e.target = op;
            const fire = function (l, e) {
                return function () { _fireListener(l, e); };
            };
            // @ts-ignore
            for (const l in listeners[e.type]) {
                // @ts-ignore
                setTimeout(fire(listeners[e.type][l], e), 0);
            }
        };
        this.postMessage = function M_postMessage(method, args) {
            console.log("TRACE Entered M_postMessage");
            // was: Create a PCMessage
            const msg = PCMessage.create({
                // @ts-ignore
                opid: myOpid,
                toBack: true,
                method: method,
                args: args
            });
            // Post it to the back end
            myBackend === null || myBackend === void 0 ? void 0 : myBackend.postMessage(msg, myOrigin);
        };
        const that = this;
        _fireListener = function M_fireListener(listener, e) {
            console.log("TRACE Entered M_fireListener");
            if (typeof (listener) === 'function') {
                listener(e);
            }
            else if ((typeof (listener) === 'object') &&
                // @ts-ignore
                listener.hasOwnProperty('handleEvent') &&
                // @ts-ignore
                (typeof (listener.handleEvent) === 'function')) {
                // @ts-ignore
                listener.handleEvent(e);
            }
            else {
                console.log("not really firing; listener of unknown type", typeof (listener));
                console.log(listener);
            }
        };
        _handleMessage = function M_handleMessage(e) {
            // Validate message
            const msg = e.data;
            if (!PCMessage.valid(msg)) {
                return;
            }
            if (!msg.toFront) {
                return;
            }
            if (!msg.hasOwnProperty('opid') || (msg.opid !== myOpid)) {
                return;
            }
            console.log("TRACE Entered M_handleMessage");
            // If the message has a result, cache it
            if (typeof (msg.result) !== 'undefined') {
                setResult(msg.result);
                console.log('_handleMessage delivered result: ' + JSON.stringify(msg.result));
            }
            // Extract and dispatch event
            if (msg.event) {
                that.dispatchEvent(msg.event);
            }
            // Any other things to handle here?  Self-destruct?
        };
        // Register for events coming back
        window.addEventListener('message', _handleMessage);
    }
    //----------------------------------------------------------------------
    const createCryptoOp = function pc_createCryptoOp(type, algorithm, key, signature, buffer) {
        console.log("TRACE Entered pc_createCryptoOp");
        const op = {};
        let result = null;
        // @ts-ignore
        const messenger = new Messenger(op, function (x) { result = x; });
        // @ts-ignore
        op.addEventListener = function (type, listener, useCapture) {
            messenger.addEventListener(type, listener, useCapture);
        };
        // @ts-ignore
        op.removeEventListener = function (type, listener) {
            messenger.removeEventListener(type, listener);
        };
        // @ts-ignore
        op.dispatchEvent = function (e) { messenger.dispatchEvent(e); };
        // define the CryptoOperation interface
        // attributes
        Object.defineProperty(op, 'algorithm', {
            enumerable: true,
            get: function get() { return algorithm; }
        });
        Object.defineProperty(op, 'key', {
            enumerable: true,
            get: function get() { return key; }
        });
        Object.defineProperty(op, 'result', {
            enumerable: true,
            get: function get() { return result; }
        });
        // the methods of the CryptoOperation interface
        // @ts-ignore
        op.process = function CO_process(buffer) {
            messenger.postMessage("process", {
                buffer: buffer,
            });
        };
        // @ts-ignore
        op.finish = function CO_process() {
            messenger.postMessage("finish");
        };
        // @ts-ignore
        op.abort = function CO_process() {
            messenger.postMessage("abort");
        };
        // callback methods
        Object.defineProperty(op, 'onabort', {
            enumerable: true,
            configurable: false,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('abort', listener, false);
            }
        });
        Object.defineProperty(op, 'onerror', {
            enumerable: true,
            configurable: false,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('error', listener, false);
            }
        });
        Object.defineProperty(op, 'onprogress', {
            enumerable: true,
            configurable: false,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('progress', listener, false);
            }
        });
        Object.defineProperty(op, 'oncomplete', {
            enumerable: true,
            configurable: true,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('complete', listener, false);
            }
        });
        // Register for events coming back
        window.addEventListener('message', messenger._handleMessage);
        // Create an implementation object
        messenger.postMessage('init', { type: type });
        // Kick off the operation
        if (!buffer) {
            buffer = new Uint8Array(0);
        }
        const args = {
            algorithm: algorithm,
            key: key,
            signature: signature,
            buffer: buffer,
        };
        messenger.postMessage("create", args);
        return op;
    };
    //--------------------------------------------------------------------------
    const createKeyOp = function pc_createKeyOp(type, format, keyData, algorithm, extractable, keyUsages, baseKey, derivedKeyType, key) {
        console.log("TRACE Entered pc_createKeyOp");
        let op = {}, result = null, 
        // @ts-ignore
        messenger = new Messenger(op, function (x) { result = x; });
        // @ts-ignore
        op.addEventListener = function (type, listener, useCapture) {
            messenger.addEventListener(type, listener, useCapture);
        };
        // @ts-ignore
        op.removeEventListener = function (type, listener) {
            messenger.removeEventListener(type, listener);
        };
        // @ts-ignore
        op.dispatchEvent = function (e) { messenger.dispatchEvent(e); };
        // define the KeyOperation interface
        // attributes
        Object.defineProperty(op, 'result', {
            enumerable: true,
            get: function get() { return result; }
        });
        // callback methods
        Object.defineProperty(op, 'onerror', {
            enumerable: true,
            configurable: false,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('error', listener, false);
            }
        });
        Object.defineProperty(op, 'oncomplete', {
            enumerable: true,
            configurable: true,
            set: function (listener) {
                // @ts-ignore
                op.addEventListener('complete', listener, false);
            }
        });
        // Register for events coming back
        window.addEventListener('message', messenger._handleMessage);
        // Create an implementation object
        messenger.postMessage('init', { type: type });
        // Kick off the operation
        const args = {
            format: format,
            keyData: keyData,
            algorithm: algorithm,
            extractable: extractable,
            keyUsages: keyUsages,
            baseKey: baseKey,
            derivedKeyType: derivedKeyType,
            key: key,
        };
        messenger.postMessage(type, args);
        return op;
    };
    //--------------------------------------------------------------------------
    // Add a flag to indicate we're using the polyfill, and not
    // a mirrored native implementation
    me.polyfill = true;
    //--------------------------------------------------------------------------
    // Add methods to window.crypto
    me.encrypt = function pc_encrypt(algorithm, key, buffer) {
        return createCryptoOp('encrypt', algorithm, key, null, buffer);
    };
    me.decrypt = function pc_decrypt(algorithm, key, buffer) {
        return createCryptoOp('decrypt', algorithm, key, null, buffer);
    };
    me.sign = function pc_sign(algorithm, key, buffer) {
        return createCryptoOp('sign', algorithm, key, null, buffer);
    };
    me.verify = function pc_verify(algorithm, key, signature, buffer) {
        return createCryptoOp('verify', algorithm, key, signature, buffer);
    };
    me.digest = function pc_digest(algorithm, buffer) {
        return createCryptoOp('digest', algorithm, null, null, buffer);
    };
    me.generateKey = function pc_generateKey(algorithm, extractable, keyUsages) {
        return createKeyOp('generate', null, null, algorithm, extractable, keyUsages);
    };
    me.deriveKey = function pc_deriveKey(algorithm, baseKey, derivedKeyType, extractable, keyUsages) {
        return createKeyOp('derive', null, null, algorithm, extractable, keyUsages, baseKey, derivedKeyType);
    };
    me.importKey = function pc_importKey(format, keyData, algorithm, extractable, keyUsages) {
        return createKeyOp('import', format, keyData, algorithm, extractable, keyUsages);
    };
    me.exportKey = function pc_exportKey(format, key) {
        return createKeyOp('export', format, null, null, null, null, null, null, key);
    };
    //--------------------------------------------------------------------------
    // Wait for the code from the polycrypt backend to be loaded.
    // Call the callback so the application's script can start using polycrypt.
    me.onalive = function () { };
    handleAlive = function crypto_handleAlive(e) {
        if ('polycrypt backend is alive' === e.data) {
            if (me.onalive && typeof (me.onalive) === 'function') {
                me.onalive();
            }
            window.removeEventListener('message', crypto_handleAlive, false);
        }
    };
    window.addEventListener('message', handleAlive, false);
    //--------------------------------------------------------------------------
    return me;
})();
