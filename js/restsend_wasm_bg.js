let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}


const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

let WASM_VECTOR_LEN = 0;

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedBigInt64Memory0 = null;

function getBigInt64Memory0() {
    if (cachedBigInt64Memory0 === null || cachedBigInt64Memory0.byteLength === 0) {
        cachedBigInt64Memory0 = new BigInt64Array(wasm.memory.buffer);
    }
    return cachedBigInt64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}
function __wbg_adapter_52(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__ha889253f048f7c0f(arg0, arg1);
}

function __wbg_adapter_55(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__h3e938dd7c54ea8f4(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_62(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__hebb5522d251d5b58(arg0, arg1, addHeapObject(arg2));
}

let cachedUint32Memory0 = null;

function getUint32Memory0() {
    if (cachedUint32Memory0 === null || cachedUint32Memory0.byteLength === 0) {
        cachedUint32Memory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32Memory0;
}

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    const mem = getUint32Memory0();
    for (let i = 0; i < array.length; i++) {
        mem[ptr / 4 + i] = addHeapObject(array[i]);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}
/**
* @param {string | undefined} [level]
*/
export function setLogging(level) {
    var ptr0 = isLikeNone(level) ? 0 : passStringToWasm0(level, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.setLogging(ptr0, len0);
}

/**
* Signin with userId and password or token
* @param {string} endpoint
* @param {string} userId
* @param {string | undefined} [password]
* @param {string | undefined} [token]
* @returns {Promise<any>}
*/
export function signin(endpoint, userId, password, token) {
    const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    var ptr2 = isLikeNone(password) ? 0 : passStringToWasm0(password, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len2 = WASM_VECTOR_LEN;
    var ptr3 = isLikeNone(token) ? 0 : passStringToWasm0(token, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len3 = WASM_VECTOR_LEN;
    const ret = wasm.signin(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
    return takeObject(ret);
}

/**
* Signup with userId and password
* @param {string} endpoint
* @param {string} userId
* @param {string} password
* @returns {Promise<any>}
*/
export function signup(endpoint, userId, password) {
    const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(password, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.signup(ptr0, len0, ptr1, len1, ptr2, len2);
    return takeObject(ret);
}

/**
* Logout with token
* @param {string} endpoint
* @param {string} token
* @returns {Promise<void>}
*/
export function logout(endpoint, token) {
    const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(token, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.logout(ptr0, len0, ptr1, len1);
    return takeObject(ret);
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_322(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h69279b9161a1265d(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function notDefined(what) { return () => { throw new Error(`${what} is not defined`); }; }
/**
*/
export class Client {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_client_free(ptr);
    }
    /**
    * @param {any} info
    */
    constructor(info) {
        const ret = wasm.client_new(addHeapObject(info));
        this.__wbg_ptr = ret >>> 0;
        return this;
    }
    /**
    * get the current connection status
    * return: connecting, connected, net_broken, shutdown
    * @returns {string}
    */
    get connectionStatus() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.client_connectionStatus(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {Promise<void>}
    */
    shutdown() {
        const ret = wasm.client_shutdown(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<void>}
    */
    connect() {
        const ret = wasm.client_connect(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    *
    * Send message with content
    * # Arguments
    * * `topicId` - The topic id
    * * `content` - The content Object
    *     * `type` String - The content type, must be [text, image, video, audio, file, YOUR_CUSTOM_TYPE]
    *     * `text` String - The text message
    *     * `attachment` Object - The attachment object
    *     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    *     * `thumbnail` Object - The thumbnail object, only for video and image, optional
    *     * `size` Number - The size of the content, only for file, optional
    *     * `placeholder` String - The placeholder of the content, optional
    *     * `width` Number - The width of the content, only for image/video, optional
    *     * `height` Number - The height of the content, only for image/video, optional
    *     * `reply` String - The reply message id, optional
    *     * `mentions` Array - Mention to users, optional
    *     * `mentionsAll` Boolean - Mention to all users, optional
    * * `option` - The send option
    * # Return
    * The message id
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * await client.doSend(topicId, {
    *     type: 'wx.text',
    *     text: 'hello',
    * }, {
    *     mentions: undefined, // The mention user id list, optional
    *     mentionAll:  false, // Mention all users, optional
    *     reply:  undefined, // The reply message id, optional
    *     onsent:  () => {}, // The callback when message sent
    *     onprogress:  (progress:Number, total:Number)  =>{}, // The callback when message sending progress
    *     onack:  (req:ChatRequest)  => {}, // The callback when message acked
    *     onfail:  (reason:String)  => {} // The callback when message failed
    * });
    * ```
    * @param {string} topicId
    * @param {any} content
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSend(topicId, content, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSend(this.__wbg_ptr, ptr0, len0, addHeapObject(content), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send typing status
    * # Arguments
    * * `topicId` - The topic id
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    doTyping(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doTyping(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Recall message
    * # Arguments
    * * `topicId` - The topic id
    * * `messageId` - The message id
    * @param {string} topicId
    * @param {string} messageId
    * @param {any} option
    * @returns {Promise<string>}
    */
    doRecall(topicId, messageId, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(messageId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_doRecall(this.__wbg_ptr, ptr0, len0, ptr1, len1, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send voice message
    * # Arguments
    * * `topicId` - The topic id
    * * `attachment` - The attachment object
    * * `option` - The send option
    *     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    *     * `mentions` Array - The mention user id list, optional
    *     * `mentionAll` boolean, // Mention all users, optional
    *     * `reply` String - The reply message id, optional
    * # Return
    * The message id
    * @param {string} topicId
    * @param {any} attachment
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendVoice(topicId, attachment, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendVoice(this.__wbg_ptr, ptr0, len0, addHeapObject(attachment), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send video message
    * # Arguments
    * * `topicId` - The topic id
    * * `attachment` - The attachment object
    * * `option` - The send option
    *    * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
    *    * `mentions` Array - The mention user id list, optional
    *    * `mentionAll` boolean, // Mention all users, optional
    *    * `reply` String - The reply message id, optional
    * # Return
    * The message id
    * @param {string} topicId
    * @param {any} attachment
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendVideo(topicId, attachment, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendVideo(this.__wbg_ptr, ptr0, len0, addHeapObject(attachment), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send file message
    * # Arguments
    * * `topicId` - The topic id
    * * `attachment` - The attachment object
    * * `option` - The send option
    *    * `size` Number - The size of the content, only for file, optional
    *    * `mentions` Array - The mention user id list, optional
    *    * `mentionAll` boolean, // Mention all users, optional
    *    * `reply` String - The reply message id, optional
    * # Return
    * The message id
    * @param {string} topicId
    * @param {any} attachment
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendFile(topicId, attachment, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendFile(this.__wbg_ptr, ptr0, len0, addHeapObject(attachment), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send location message
    * # Arguments
    * * `topicId` - The topic id
    * * `latitude` - The latitude
    * * `longitude` - The longitude
    * * `address` - The address
    * * `option` - The send option
    *   * `mentions` Array - The mention user id list, optional
    *   * `mentionAll` boolean, // Mention all users, optional
    *   * `reply` String - The reply message id, optional
    * # Return
    * The message id
    * @param {string} topicId
    * @param {string} latitude
    * @param {string} longitude
    * @param {string} address
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendLocation(topicId, latitude, longitude, address, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(latitude, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(longitude, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(address, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendLocation(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send link message
    * # Arguments
    * * `topicId` - The topic id
    * * `url` - The url
    * * `option` - The send option
    *  * `placeholder` String - The placeholder of the content, optional
    *  * `mentions` Array - The mention user id list, optional
    *  * `mentionAll` boolean, // Mention all users, optional
    *  * `reply` String - The reply message id, optional
    * # Return
    * The message id
    * @param {string} topicId
    * @param {string} url
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendLink(topicId, url, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendLink(this.__wbg_ptr, ptr0, len0, ptr1, len1, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send invite message
    * # Arguments
    * * `topicId` - The topic id
    * * `logIds` Array - The log id list
    * * `option` - The send option
    * # Return
    * The message id
    * @param {string} topicId
    * @param {(string)[]} logIds
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendLogs(topicId, logIds, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(logIds, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendLogs(this.__wbg_ptr, ptr0, len0, ptr1, len1, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Send text message
    * # Arguments
    * * `topicId` - The topic id
    * * `text` - The text message
    * * `option` - The send option
    * # Return
    * The message id
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * await client.sendText(topicId, text, {
    *     mentions: [] || undefined, // The mention user id list, optional
    *     reply: String || undefined, - The reply message id, optional
    *     onsent:  () => {},
    *     onprogress:  (progress:Number, total:Number)  =>{},
    *     onack:  (req:ChatRequest)  => {},
    *     onfail:  (reason:String)  => {}
    * });
    * ```
    * @param {string} topicId
    * @param {string} text
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendText(topicId, text, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendText(this.__wbg_ptr, ptr0, len0, ptr1, len1, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    *
    * Send image message
    * # Arguments
    * * `topicId` - The topic id
    * * `attachment` - The attachment object
    *     * `file` File - The file object
    *     * `url` String  - The file name
    * * `option` - The send option
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * await client.sendImage(topicId, {file:new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' })}, {});
    * ```
    * @param {string} topicId
    * @param {any} attachment
    * @param {any} option
    * @returns {Promise<string>}
    */
    doSendImage(topicId, attachment, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendImage(this.__wbg_ptr, ptr0, len0, addHeapObject(attachment), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Set the callback when connection connected
    * @param {any} cb
    */
    set onconnected(cb) {
        wasm.client_set_onconnected(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when connection connecting
    * @param {any} cb
    */
    set onconnecting(cb) {
        wasm.client_set_onconnecting(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when connection token expired
    * @param {any} cb
    */
    set ontokenexpired(cb) {
        wasm.client_set_ontokenexpired(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when connection broken
    * # Arguments
    * * `reason` String - The reason of the connection broken
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onnetbroken = (reason) => {
    * console.log(reason);
    * }
    * ```
    * @param {any} cb
    */
    set onbroken(cb) {
        wasm.client_set_onbroken(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when kickoff by other client
    * # Arguments
    * * `reason` String - The reason of the kickoff
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onkickoff = (reason) => {
    * console.log(reason);
    * }
    * ```
    * @param {any} cb
    */
    set onkickoff(cb) {
        wasm.client_set_onkickoff(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when receive system request
    * # Arguments
    *  * `req` - The request object, the return value is the response object
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onsystemrequest = (req) => {
    *    if (req.type === 'get') {
    *       return {type:'resp', code: 200}
    *   }
    * }
    * ```
    * @param {any} cb
    */
    set onsystemrequest(cb) {
        wasm.client_set_onsystemrequest(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when receive unknown request
    * # Arguments
    *  * `req` - The request object, the return value is the response object
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onunknownrequest = (req) => {
    *   if (req.type === 'get') {
    *      return {type:'resp', code: 200}
    *  }
    * }
    * @param {any} cb
    */
    set onunknownrequest(cb) {
        wasm.client_set_onunknownrequest(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when receive typing event
    * # Arguments
    * * `topicId` String - The topic id
    * * `message` ChatRequest - The message
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.ontyping = (topicId, message) => {
    *  console.log(topicId, message);
    * }
    * ```
    * @param {any} cb
    */
    set ontopictyping(cb) {
        wasm.client_set_ontopictyping(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when receive new message
    * # Arguments
    * * `topicId` String - The topic id
    * * `message` ChatRequest - The message
    * # Return
    * * `true` - If return true, will send `has read` to server
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.ontopicmessage = (topicId, message) => {
    * console.log(topicId, message);
    * return true;
    * }
    * ```
    * @param {any} cb
    */
    set ontopicmessage(cb) {
        wasm.client_set_ontopicmessage(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when receive read event
    * # Arguments
    * * `topicId` String - The topic id
    * * `message` ChatRequest - The message
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.ontopicread = (topicId, message) => {
    * console.log(topicId, message);
    * }
    * ```
    * @param {any} cb
    */
    set ontopicread(cb) {
        wasm.client_set_ontopicread(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when conversations updated
    * # Arguments
    * * `conversations` - The conversation list
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onconversationsupdated = (conversations) => {
    * console.log(conversations);
    * }
    * ```
    * @param {any} cb
    */
    set onconversationsupdated(cb) {
        wasm.client_set_onconversationsupdated(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Set the callback when conversations removed
    * # Arguments
    * * `conversationId` - The conversation id
    * # Example
    * ```javascript
    * const client = new Client(info);
    * await client.connect();
    * client.onconversationsremoved = (conversationId) => {
    * console.log(conversationId);
    * }
    * ```
    * @param {any} cb
    */
    set onconversationsremoved(cb) {
        wasm.client_set_onconversationsremoved(this.__wbg_ptr, addHeapObject(cb));
    }
    /**
    * Create a new chat with userId
    * return: Conversation
    * @param {string} userId
    * @returns {Promise<any>}
    */
    createChat(userId) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_createChat(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Clean history of a conversation
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    cleanMessages(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_cleanMessages(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Remove messages from a conversation
    * @param {string} topicId
    * @param {(string)[]} chatIds
    * @returns {Promise<void>}
    */
    removeMessages(topicId, chatIds) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(chatIds, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_removeMessages(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Sync chat logs from server
    * #Arguments
    * * `topicId` - topic id
    * * `lastSeq` - Number, last seq
    * * `option` - option
    *     * `limit` - limit
    *     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
    *     * `onerror` - onerror callback -> function (error: String)
    * @param {string} topicId
    * @param {any} lastSeq
    * @param {any} option
    * @returns {Promise<void>}
    */
    syncChatLogs(topicId, lastSeq, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_syncChatLogs(this.__wbg_ptr, ptr0, len0, addHeapObject(lastSeq), addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Sync conversations from server
    * #Arguments
    * * `option` - option
    *    * `limit` - limit
    *    * `updatedAt` String - updated_at optional
    *    * `onsuccess` - onsuccess callback -> function (updated_at:String, count: u32)
    *         - updated_at: last updated_at
    *         - count: count of conversations, if count == limit, there may be more conversations, you can call syncConversations again with updated_at, stop when count < limit
    *    * `onerror` - onerror callback -> function (error: String)
    * @param {any} option
    * @returns {Promise<void>}
    */
    syncConversations(option) {
        const ret = wasm.client_syncConversations(this.__wbg_ptr, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Get conversation by topicId
    * #Arguments
    * * `topicId` - topic id
    * return: Conversation or null
    * @param {string} topicId
    * @returns {any}
    */
    getConversation(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getConversation(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Remove conversation by topicId
    * #Arguments
    * * `topicId` - topic id
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    removeConversation(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_removeConversation(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Set conversation remark
    * #Arguments
    * * `topicId` - topic id
    * * `remark` - remark
    * @param {string} topicId
    * @param {string | undefined} [remark]
    * @returns {Promise<any>}
    */
    setConversationRemark(topicId, remark) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(remark) ? 0 : passStringToWasm0(remark, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationRemark(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Set conversation sticky by topicId
    * #Arguments
    * * `topicId` - topic id
    * * `sticky` - sticky
    * @param {string} topicId
    * @param {boolean} sticky
    * @returns {Promise<any>}
    */
    setConversationSticky(topicId, sticky) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationSticky(this.__wbg_ptr, ptr0, len0, sticky);
        return takeObject(ret);
    }
    /**
    * Set conversation mute by topicId
    * #Arguments
    * * `topicId` - topic id
    * * `mute` - mute
    * @param {string} topicId
    * @param {boolean} mute
    * @returns {Promise<any>}
    */
    setConversationMute(topicId, mute) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationMute(this.__wbg_ptr, ptr0, len0, mute);
        return takeObject(ret);
    }
    /**
    * Set conversation read by topicId
    * #Arguments
    * * `topicId` - topic id
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    setConversationRead(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationRead(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Set conversation tags
    * #Arguments
    * * `topicId` - topic id
    * * `tags` - tags is array of Tag:
    *     - id - string
    *     - type - string
    *     - label - string
    * @param {string} topicId
    * @param {any} tags
    * @returns {Promise<any>}
    */
    setConversationTags(topicId, tags) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationTags(this.__wbg_ptr, ptr0, len0, addHeapObject(tags));
        return takeObject(ret);
    }
    /**
    * Set conversation extra
    * #Arguments
    * * `topicId` - topic id
    * # `extra` - extra
    * # Return: Conversation
    * @param {string} topicId
    * @param {any} extra
    * @returns {Promise<any>}
    */
    setConversationExtra(topicId, extra) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationExtra(this.__wbg_ptr, ptr0, len0, addHeapObject(extra));
        return takeObject(ret);
    }
    /**
    * Filter conversation with options
    * #Arguments
    * * `predicate` - filter predicate
    *     -> return true to keep the conversation
    * #Return Array of Conversation
    * #Example
    * ```js
    * const conversations = client.filterConversation((c) => {
    *    return c.remark === 'hello'
    * })
    * ```
    * #Example
    * ```js
    * const conversations = client.filterConversation((c) => {
    *   return c.remark === 'hello' && c.tags && c.tags.some(t => t.label === 'hello')
    * })
    * @param {any} predicate
    * @returns {Promise<any>}
    */
    filterConversation(predicate) {
        const ret = wasm.client_filterConversation(this.__wbg_ptr, addHeapObject(predicate));
        return takeObject(ret);
    }
    /**
    * Create a new topic
    * #Arguments
    *   name: String,
    *  icon: String,
    * #Return
    * * `Topic` || `undefined`
    * @param {(string)[]} members
    * @param {string | undefined} [name]
    * @param {string | undefined} [icon]
    * @returns {Promise<any>}
    */
    createTopic(members, name, icon) {
        const ptr0 = passArrayJsValueToWasm0(members, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(name) ? 0 : passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(icon) ? 0 : passStringToWasm0(icon, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_createTopic(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * Join a topic
    * #Arguments
    * * `topicId` - topic id
    * * `message` - message
    * * `source` - source
    * @param {string} topicId
    * @param {string | undefined} [message]
    * @param {string | undefined} [source]
    * @returns {Promise<void>}
    */
    joinTopic(topicId, message, source) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(message) ? 0 : passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(source) ? 0 : passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_joinTopic(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * Add user into topic
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * #Return
    * * `TopicMember` || `undefined`
    * @param {string} topicId
    * @param {string} userId
    * @returns {Promise<any>}
    */
    addMember(topicId, userId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_addMember(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Get topic info
    * #Arguments
    * * `topicId` - topic id
    * #Return
    * * `Topic` || `undefined`
    * @param {string} topicId
    * @returns {Promise<any>}
    */
    getTopic(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getTopic(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Get topic admins
    * #Arguments
    * * `topicId` - topic id
    * #Return
    * * `Vec<User>` || `undefined`
    * @param {string} topicId
    * @returns {Promise<any>}
    */
    getTopicAdmins(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getTopicAdmins(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Get topic owner
    * #Arguments
    * * `topicId` - topic id
    * #Return
    * * `User` || `undefined`
    * @param {string} topicId
    * @returns {Promise<any>}
    */
    getTopicOwner(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getTopicOwner(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Get topic members
    * #Arguments
    * * `topicId` - topic id
    * * `updatedAt` - updated_at
    * * `limit` - limit
    * #Return
    * * `ListUserResult` || `undefined`
    * @param {string} topicId
    * @param {string} updatedAt
    * @param {number} limit
    * @returns {Promise<any>}
    */
    getTopicMembers(topicId, updatedAt, limit) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(updatedAt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_getTopicMembers(this.__wbg_ptr, ptr0, len0, ptr1, len1, limit);
        return takeObject(ret);
    }
    /**
    * Get topic knocks
    * #Arguments
    * * `topicId` - topic id
    * #Return
    * * `Vec<TopicKnock>`
    * @param {string} topicId
    * @returns {Promise<any>}
    */
    getTopicKnocks(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getTopicKnocks(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Update topic info
    * #Arguments
    * * `topicId` - topic id
    * * `option` - option
    *     * `name` - String
    *     * `icon` - String (url) or base64
    * @param {string} topicId
    * @param {any} option
    * @returns {Promise<void>}
    */
    updateTopic(topicId, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_updateTopic(this.__wbg_ptr, ptr0, len0, addHeapObject(option));
        return takeObject(ret);
    }
    /**
    * Update topic notice
    * #Arguments
    * * `topicId` - topic id
    * * `text` - notice text
    * @param {string} topicId
    * @param {string} text
    * @returns {Promise<void>}
    */
    updateTopicNotice(topicId, text) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_updateTopicNotice(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Silence topic
    * #Arguments
    * * `topicId` - topic id
    * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
    * @param {string} topicId
    * @param {string | undefined} [duration]
    * @returns {Promise<void>}
    */
    silentTopic(topicId, duration) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(duration) ? 0 : passStringToWasm0(duration, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_silentTopic(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Silent topic member
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
    * @param {string} topicId
    * @param {string} userId
    * @param {string | undefined} [duration]
    * @returns {Promise<void>}
    */
    silentTopicMember(topicId, userId, duration) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(duration) ? 0 : passStringToWasm0(duration, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_silentTopicMember(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * Add topic admin
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * @param {string} topicId
    * @param {string} userId
    * @returns {Promise<void>}
    */
    addTopicAdmin(topicId, userId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_addTopicAdmin(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Remove topic admin
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * @param {string} topicId
    * @param {string} userId
    * @returns {Promise<void>}
    */
    removeTopicAdmin(topicId, userId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_removeTopicAdmin(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Transfer topic
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id to transfer, the user must be a topic member
    * @param {string} topicId
    * @param {string} userId
    * @returns {Promise<void>}
    */
    transferTopic(topicId, userId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_transferTopic(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Quit topic
    * #Arguments
    * * `topicId` - topic id
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    quitTopic(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_quitTopic(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Dismiss topic
    * #Arguments
    * * `topicId` - topic id
    * @param {string} topicId
    * @returns {Promise<void>}
    */
    dismissTopic(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_dismissTopic(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Accept topic join
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * * `memo` - accept memo
    * @param {string} topicId
    * @param {string} userId
    * @param {string | undefined} [memo]
    * @returns {Promise<void>}
    */
    acceptTopicJoin(topicId, userId, memo) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(memo) ? 0 : passStringToWasm0(memo, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_acceptTopicJoin(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * Decline topic join
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * * `message` - decline message
    * @param {string} topicId
    * @param {string} userId
    * @param {string | undefined} [message]
    * @returns {Promise<void>}
    */
    declineTopicJoin(topicId, userId, message) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(message) ? 0 : passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_declineTopicJoin(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * Remove topic member
    * #Arguments
    * * `topicId` - topic id
    * * `userId` - user id
    * @param {string} topicId
    * @param {string} userId
    * @returns {Promise<void>}
    */
    removeTopicMember(topicId, userId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_removeTopicMember(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Get user info
    * #Arguments
    * * `userId` - user id
    * * `blocking` - blocking fetch from server
    * #Return
    * User info
    * @param {string} userId
    * @param {boolean | undefined} [blocking]
    * @returns {Promise<any>}
    */
    getUser(userId, blocking) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getUser(this.__wbg_ptr, ptr0, len0, isLikeNone(blocking) ? 0xFFFFFF : blocking ? 1 : 0);
        return takeObject(ret);
    }
    /**
    * Get multiple users info
    * #Arguments
    * * `userIds` - Array of user id
    * #Return
    * Array of user info
    * @param {(string)[]} userIds
    * @returns {Promise<any>}
    */
    getUsers(userIds) {
        const ptr0 = passArrayJsValueToWasm0(userIds, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getUsers(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * Set user remark name
    * #Arguments
    * * `userId` - user id
    * * `remark` - remark name
    * @param {string} userId
    * @param {string} remark
    * @returns {Promise<void>}
    */
    setUserRemark(userId, remark) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(remark, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_setUserRemark(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Set user star
    * #Arguments
    * * `userId` - user id
    * * `star` - star
    * @param {string} userId
    * @param {boolean} star
    * @returns {Promise<void>}
    */
    setUserStar(userId, star) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setUserStar(this.__wbg_ptr, ptr0, len0, star);
        return takeObject(ret);
    }
    /**
    * Set user block
    * #Arguments
    * * `userId` - user id
    * * `block` - block
    * @param {string} userId
    * @param {boolean} block
    * @returns {Promise<void>}
    */
    setUserBlock(userId, block) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setUserBlock(this.__wbg_ptr, ptr0, len0, block);
        return takeObject(ret);
    }
    /**
    * Set allow guest chat
    * #Arguments
    * * `allow` - allow
    * @param {boolean} allow
    * @returns {Promise<void>}
    */
    setAllowGuestChat(allow) {
        const ret = wasm.client_setAllowGuestChat(this.__wbg_ptr, allow);
        return takeObject(ret);
    }
}
/**
*/
export class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr);
    }
    /**
    * @returns {string}
    */
    get type() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.intounderlyingbytesource_type(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {number}
    */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @param {any} controller
    */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
    * @param {any} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}
/**
*/
export class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr);
    }
    /**
    * @param {any} chunk
    * @returns {Promise<any>}
    */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, addHeapObject(chunk));
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} reason
    * @returns {Promise<any>}
    */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, addHeapObject(reason));
        return takeObject(ret);
    }
}
/**
*/
export class IntoUnderlyingSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr);
    }
    /**
    * @param {any} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
}
/**
* Raw options for [`pipeTo()`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/pipeTo).
*/
export class PipeOptions {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_pipeoptions_free(ptr);
    }
    /**
    * @returns {boolean}
    */
    get preventClose() {
        const ret = wasm.pipeoptions_preventClose(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    get preventCancel() {
        const ret = wasm.pipeoptions_preventCancel(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    get preventAbort() {
        const ret = wasm.pipeoptions_preventAbort(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
    * @returns {AbortSignal | undefined}
    */
    get signal() {
        const ret = wasm.pipeoptions_signal(this.__wbg_ptr);
        return takeObject(ret);
    }
}
/**
*/
export class QueuingStrategy {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_queuingstrategy_free(ptr);
    }
    /**
    * @returns {number}
    */
    get highWaterMark() {
        const ret = wasm.queuingstrategy_highWaterMark(this.__wbg_ptr);
        return ret;
    }
}
/**
* Raw options for [`getReader()`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/getReader).
*/
export class ReadableStreamGetReaderOptions {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_readablestreamgetreaderoptions_free(ptr);
    }
    /**
    * @returns {any}
    */
    get mode() {
        const ret = wasm.readablestreamgetreaderoptions_mode(this.__wbg_ptr);
        return takeObject(ret);
    }
}

export function __wbindgen_object_drop_ref(arg0) {
    takeObject(arg0);
};

export function __wbindgen_string_new(arg0, arg1) {
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

export function __wbindgen_is_function(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
};

export function __wbindgen_number_get(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'number' ? obj : undefined;
    getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbindgen_string_get(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbindgen_boolean_get(arg0) {
    const v = getObject(arg0);
    const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
    return ret;
};

export function __wbindgen_object_clone_ref(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
};

export function __wbindgen_is_bigint(arg0) {
    const ret = typeof(getObject(arg0)) === 'bigint';
    return ret;
};

export function __wbindgen_bigint_from_i64(arg0) {
    const ret = arg0;
    return addHeapObject(ret);
};

export function __wbindgen_jsval_eq(arg0, arg1) {
    const ret = getObject(arg0) === getObject(arg1);
    return ret;
};

export function __wbindgen_is_string(arg0) {
    const ret = typeof(getObject(arg0)) === 'string';
    return ret;
};

export function __wbindgen_is_object(arg0) {
    const val = getObject(arg0);
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
};

export function __wbindgen_is_undefined(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
};

export function __wbindgen_in(arg0, arg1) {
    const ret = getObject(arg0) in getObject(arg1);
    return ret;
};

export function __wbindgen_bigint_from_u64(arg0) {
    const ret = BigInt.asUintN(64, arg0);
    return addHeapObject(ret);
};

export function __wbindgen_is_null(arg0) {
    const ret = getObject(arg0) === null;
    return ret;
};

export function __wbindgen_cb_drop(arg0) {
    const obj = takeObject(arg0).original;
    if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
    }
    const ret = false;
    return ret;
};

export function __wbindgen_error_new(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbindgen_number_new(arg0) {
    const ret = arg0;
    return addHeapObject(ret);
};

export function __wbindgen_jsval_loose_eq(arg0, arg1) {
    const ret = getObject(arg0) == getObject(arg1);
    return ret;
};

export function __wbindgen_as_number(arg0) {
    const ret = +getObject(arg0);
    return ret;
};

export function __wbg_getwithrefkey_4a92a5eca60879b9(arg0, arg1) {
    const ret = getObject(arg0)[getObject(arg1)];
    return addHeapObject(ret);
};

export function __wbg_set_9182712abebf82ef(arg0, arg1, arg2) {
    getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
};

export function __wbg_setTimeout_e85cc7fc84067d7a(arg0, arg1) {
    setTimeout(getObject(arg0), arg1 >>> 0);
};

export function __wbg_fetch_b5d6bebed1e6c2d2(arg0) {
    const ret = fetch(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_respond_8fadc5f5c9d95422(arg0, arg1) {
    getObject(arg0).respond(arg1 >>> 0);
};

export function __wbg_close_e9110ca16e2567db(arg0) {
    getObject(arg0).close();
};

export function __wbg_enqueue_d71a1a518e21f5c3(arg0, arg1) {
    getObject(arg0).enqueue(getObject(arg1));
};

export function __wbg_byobRequest_08c18cee35def1f4(arg0) {
    const ret = getObject(arg0).byobRequest;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_close_da7e6fb9d9851e5a(arg0) {
    getObject(arg0).close();
};

export function __wbg_view_231340b0dd8a2484(arg0) {
    const ret = getObject(arg0).view;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_buffer_4e79326814bdd393(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
};

export function __wbg_byteOffset_b69b0a07afccce19(arg0) {
    const ret = getObject(arg0).byteOffset;
    return ret;
};

export function __wbg_byteLength_5299848ed3264181(arg0) {
    const ret = getObject(arg0).byteLength;
    return ret;
};

export function __wbg_queueMicrotask_4d890031a6a5a50c(arg0) {
    queueMicrotask(getObject(arg0));
};

export function __wbg_queueMicrotask_adae4bc085237231(arg0) {
    const ret = getObject(arg0).queueMicrotask;
    return addHeapObject(ret);
};

export function __wbg_instanceof_Window_3e5cd1f48c152d01(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Window;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_location_176c34e89c2c9d80(arg0) {
    const ret = getObject(arg0).location;
    return addHeapObject(ret);
};

export function __wbg_fetch_693453ca3f88c055(arg0, arg1) {
    const ret = getObject(arg0).fetch(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_debug_34c9290896ec9856(arg0) {
    console.debug(getObject(arg0));
};

export function __wbg_error_e60eff06f24ab7a4(arg0) {
    console.error(getObject(arg0));
};

export function __wbg_info_d7d58472d0bab115(arg0) {
    console.info(getObject(arg0));
};

export function __wbg_log_a4530b4fe289336f(arg0) {
    console.log(getObject(arg0));
};

export function __wbg_warn_f260f49434e45e62(arg0) {
    console.warn(getObject(arg0));
};

export function __wbg_instanceof_Blob_c7124075b9fe8788(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Blob;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_size_5c0f6bdb97c23ee9(arg0) {
    const ret = getObject(arg0).size;
    return ret;
};

export function __wbg_newwithu8arraysequenceandoptions_8a6b4effbcac4a62() { return handleError(function (arg0, arg1) {
    const ret = new Blob(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_newwithstrsequenceandoptions_4806b667a908f161() { return handleError(function (arg0, arg1) {
    const ret = new Blob(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_arrayBuffer_a9d862b05aaee2f9(arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
};

export function __wbg_origin_595edc88be6e66b8() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).origin;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
}, arguments) };

export function __wbg_host_793ff88f2063bc10() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg1).host;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
}, arguments) };

export function __wbg_instanceof_File_45d4199a1f9c7f8c(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof File;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_name_bbf9c43b9611377a(arg0, arg1) {
    const ret = getObject(arg1).name;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbg_newwithstrandinit_f581dff0d19a8b03() { return handleError(function (arg0, arg1, arg2) {
    const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_message_a438a1cce45796a8(arg0, arg1) {
    const ret = getObject(arg1).message;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbg_data_ba3ea616b5392abf(arg0) {
    const ret = getObject(arg0).data;
    return addHeapObject(ret);
};

export function __wbg_instanceof_Response_4c3b1446206114d1(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Response;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_url_83a6a4f65f7a2b38(arg0, arg1) {
    const ret = getObject(arg1).url;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbg_status_d6d47ad2837621eb(arg0) {
    const ret = getObject(arg0).status;
    return ret;
};

export function __wbg_headers_24def508a7518df9(arg0) {
    const ret = getObject(arg0).headers;
    return addHeapObject(ret);
};

export function __wbg_arrayBuffer_5b2688e3dd873fed() { return handleError(function (arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_text_668782292b0bc561() { return handleError(function (arg0) {
    const ret = getObject(arg0).text();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_signal_3c701f5f40a5f08d(arg0) {
    const ret = getObject(arg0).signal;
    return addHeapObject(ret);
};

export function __wbg_new_0ae46f44b7485bb2() { return handleError(function () {
    const ret = new AbortController();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_abort_2c4fb490d878d2b2(arg0) {
    getObject(arg0).abort();
};

export function __wbg_new_7a20246daa6eec7e() { return handleError(function () {
    const ret = new Headers();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_append_aa3f462f9e2b5ff2() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments) };

export function __wbg_new_9543178e16f01733() { return handleError(function () {
    const ret = new FormData();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_append_a2eb87e422026db5() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments) };

export function __wbg_append_26434afd037ecfb1() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
    getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
}, arguments) };

export function __wbg_setonopen_1264714f7bce70f8(arg0, arg1) {
    getObject(arg0).onopen = getObject(arg1);
};

export function __wbg_setonerror_927113bb9ac197fe(arg0, arg1) {
    getObject(arg0).onerror = getObject(arg1);
};

export function __wbg_setonclose_b2fc3455ef8818f4(arg0, arg1) {
    getObject(arg0).onclose = getObject(arg1);
};

export function __wbg_setonmessage_46f324ad82067922(arg0, arg1) {
    getObject(arg0).onmessage = getObject(arg1);
};

export function __wbg_setbinaryType_68fc3c6feda7310c(arg0, arg1) {
    getObject(arg0).binaryType = takeObject(arg1);
};

export function __wbg_new_2575c598b4006174() { return handleError(function (arg0, arg1) {
    const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_send_5bf3f962e9ffe0f6() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).send(getStringFromWasm0(arg1, arg2));
}, arguments) };

export function __wbg_get_f01601b5a68d10e3(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};

export function __wbg_length_1009b1af0c481d7b(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

export function __wbg_new_ffc6d4d085022169() {
    const ret = new Array();
    return addHeapObject(ret);
};

export function __wbg_newnoargs_c62ea9419c21fbac(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_new_bfd4534b584a9593() {
    const ret = new Map();
    return addHeapObject(ret);
};

export function __wbg_next_9b877f231f476d01(arg0) {
    const ret = getObject(arg0).next;
    return addHeapObject(ret);
};

export function __wbg_next_6529ee0cca8d57ed() { return handleError(function (arg0) {
    const ret = getObject(arg0).next();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_done_5fe336b092d60cf2(arg0) {
    const ret = getObject(arg0).done;
    return ret;
};

export function __wbg_value_0c248a78fdc8e19f(arg0) {
    const ret = getObject(arg0).value;
    return addHeapObject(ret);
};

export function __wbg_iterator_db7ca081358d4fb2() {
    const ret = Symbol.iterator;
    return addHeapObject(ret);
};

export function __wbg_get_7b48513de5dc5ea4() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_call_90c26b09837aba1c() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_9fb8d994e1c0aaac() {
    const ret = new Object();
    return addHeapObject(ret);
};

export function __wbg_self_f0e34d89f33b99fd() { return handleError(function () {
    const ret = self.self;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_window_d3b084224f4774d7() { return handleError(function () {
    const ret = window.window;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_globalThis_9caa27ff917c6860() { return handleError(function () {
    const ret = globalThis.globalThis;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_global_35dfdd59a4da3e74() { return handleError(function () {
    const ret = global.global;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_set_f2740edb12e318cd(arg0, arg1, arg2) {
    getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
};

export function __wbg_isArray_74fb723e24f76012(arg0) {
    const ret = Array.isArray(getObject(arg0));
    return ret;
};

export function __wbg_push_901f3914205d44de(arg0, arg1) {
    const ret = getObject(arg0).push(getObject(arg1));
    return ret;
};

export function __wbg_instanceof_ArrayBuffer_e7d53d51371448e2(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ArrayBuffer;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_instanceof_Error_31ca8d97f188bfbc(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Error;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_new_a64e3f2afc2cf2f8(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_message_55b9ea8030688597(arg0) {
    const ret = getObject(arg0).message;
    return addHeapObject(ret);
};

export function __wbg_call_5da1969d7cd31ccd() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_call_9079ecd7da811539() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_set_d257c6f2da008627(arg0, arg1, arg2) {
    const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};

export function __wbg_isSafeInteger_f93fde0dca9820f8(arg0) {
    const ret = Number.isSafeInteger(getObject(arg0));
    return ret;
};

export function __wbg_getTime_9272be78826033e1(arg0) {
    const ret = getObject(arg0).getTime();
    return ret;
};

export function __wbg_getTimezoneOffset_e742a5098e2c04d3(arg0) {
    const ret = getObject(arg0).getTimezoneOffset();
    return ret;
};

export function __wbg_new_d77fabdc03b9edd6(arg0) {
    const ret = new Date(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_new0_622c21a64f3d83ea() {
    const ret = new Date();
    return addHeapObject(ret);
};

export function __wbg_instanceof_Object_702c4990f4c3db8d(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Object;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_entries_9e2e2aa45aa5094a(arg0) {
    const ret = Object.entries(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_toString_e302283d4ca65316(arg0) {
    const ret = getObject(arg0).toString();
    return addHeapObject(ret);
};

export function __wbg_new_60f57089c7563e81(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return __wbg_adapter_322(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = state0.b = 0;
    }
};

export function __wbg_resolve_6e1c6553a82f85b7(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_then_3ab08cd4fbb91ae9(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_then_8371cc12cfedc5a2(arg0, arg1, arg2) {
    const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};

export function __wbg_buffer_a448f833075b71ba(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
};

export function __wbg_newwithbyteoffsetandlength_d0482f893617af71(arg0, arg1, arg2) {
    const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
    return addHeapObject(ret);
};

export function __wbg_new_8f67e318f15d7254(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_set_2357bf09366ee480(arg0, arg1, arg2) {
    getObject(arg0).set(getObject(arg1), arg2 >>> 0);
};

export function __wbg_length_1d25fa9e4ac21ce7(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

export function __wbg_instanceof_Uint8Array_bced6f43aed8c1aa(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Uint8Array;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_byteLength_af7bdd61ff8ad011(arg0) {
    const ret = getObject(arg0).byteLength;
    return ret;
};

export const __wbg_random_5f4d7bf63216a9ad = typeof Math.random == 'function' ? Math.random : notDefined('Math.random');

export function __wbg_deleteProperty_8c212ef4944e69d8() { return handleError(function (arg0, arg1) {
    const ret = Reflect.deleteProperty(getObject(arg0), getObject(arg1));
    return ret;
}, arguments) };

export function __wbg_has_9c711aafa4b444a2() { return handleError(function (arg0, arg1) {
    const ret = Reflect.has(getObject(arg0), getObject(arg1));
    return ret;
}, arguments) };

export function __wbg_set_759f75cd92b612d2() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
    return ret;
}, arguments) };

export function __wbg_stringify_e1b19966d964d242() { return handleError(function (arg0) {
    const ret = JSON.stringify(getObject(arg0));
    return addHeapObject(ret);
}, arguments) };

export function __wbindgen_bigint_get_as_i64(arg0, arg1) {
    const v = getObject(arg1);
    const ret = typeof(v) === 'bigint' ? v : undefined;
    getBigInt64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? BigInt(0) : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbindgen_debug_string(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbindgen_throw(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

export function __wbindgen_memory() {
    const ret = wasm.memory;
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper727(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 336, __wbg_adapter_52);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper728(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 336, __wbg_adapter_55);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper729(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 336, __wbg_adapter_55);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper730(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 336, __wbg_adapter_55);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2139(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 707, __wbg_adapter_62);
    return addHeapObject(ret);
};

