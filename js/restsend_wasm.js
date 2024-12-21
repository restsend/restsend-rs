let wasm;

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let WASM_VECTOR_LEN = 0;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

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
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

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
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

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

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_3.get(state.dtor)(state.a, state.b)
});

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
                wasm.__wbindgen_export_3.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
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
    if (builtInMatches && builtInMatches.length > 1) {
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

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    const mem = getDataViewMemory0();
    for (let i = 0; i < array.length; i++) {
        mem.setUint32(ptr + 4 * i, addHeapObject(array[i]), true);
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

function __wbg_adapter_52(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h00e4cadab5e5fca8(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_55(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hd46b0fab04539b04(arg0, arg1);
}

function __wbg_adapter_64(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h66556920b1f35d36(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_432(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h3767371f6ec92a1e(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];

const __wbindgen_enum_IdbCursorDirection = ["next", "nextunique", "prev", "prevunique"];

const __wbindgen_enum_IdbTransactionMode = ["readonly", "readwrite", "versionchange", "readwriteflush", "cleanup"];

const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];

const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];

const ClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_client_free(ptr >>> 0, 1));

export class Client {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_client_free(ptr, 0);
    }
    /**
     * Create a new client
     * # Arguments
     * * `info` - AuthInfo
     * * `db_name` - database name (optional), create an indexeddb when set it
     * @param {any} info
     * @param {string | undefined} [db_name]
     */
    constructor(info, db_name) {
        var ptr0 = isLikeNone(db_name) ? 0 : passStringToWasm0(db_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_new(addHeapObject(info), ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        ClientFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * get the current connection status
     * return: connecting, connected, broken, shutdown
     * @returns {string}
     */
    get connectionStatus() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.client_connectionStatus(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * connect immediately if the connection is broken
     */
    app_active() {
        wasm.client_app_active(this.__wbg_ptr);
    }
    /**
     * set the keepalive interval with seconds
     * @param {number} secs
     */
    set keepalive(secs) {
        wasm.client_set_keepalive(this.__wbg_ptr, secs);
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
     *     onattachmentupload:  (result:Upload) => { }, // The callback when attachment uploaded, return the Content object to replace the original content
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
     * @param {string} sourceTopicId
     * @param {(string)[]} logIds
     * @param {any} option
     * @returns {Promise<string>}
     */
    doSendLogs(topicId, sourceTopicId, logIds, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(sourceTopicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArrayJsValueToWasm0(logIds, wasm.__wbindgen_malloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.client_doSendLogs(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, addHeapObject(option));
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
     * Update sent chat message's extra
     * # Arguments
     * * `topicId` - The topic id
     * * `chatId` - The chat id
     * * `extra` - The extra, optional
     * * `option` - The send option
     * # Return
     * The message id
     * @param {string} topicId
     * @param {string} chatId
     * @param {any} extra
     * @param {any} option
     * @returns {Promise<string>}
     */
    doUpdateExtra(topicId, chatId, extra, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(chatId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_doUpdateExtra(this.__wbg_ptr, ptr0, len0, ptr1, len1, addHeapObject(extra), addHeapObject(option));
        return takeObject(ret);
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
     *     * `ensureConversationVersion` - ensure conversation version, default false
     *     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
     *     * `onerror` - onerror callback -> function (error: String)
     * @param {string} topicId
     * @param {number | undefined} lastSeq
     * @param {any} option
     * @returns {Promise<void>}
     */
    syncChatLogs(topicId, lastSeq, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_syncChatLogs(this.__wbg_ptr, ptr0, len0, !isLikeNone(lastSeq), isLikeNone(lastSeq) ? 0 : lastSeq, addHeapObject(option));
        return takeObject(ret);
    }
    /**
     * @param {any} logs
     * @returns {Promise<void>}
     */
    saveChatLogs(logs) {
        const ret = wasm.client_saveChatLogs(this.__wbg_ptr, addHeapObject(logs));
        return takeObject(ret);
    }
    /**
     * Sync conversations from server
     * #Arguments
     * * `option` - option
     *    * `syncLogs` - syncs logs, default false
     *    * `syncLogsLimit` - sync logs limit, per conversation, default 100
     *    * `syncLogsMaxCount` - sync logs max count, default 200
     *    * `limit` - limit
     *    * `updatedAt` String - updated_at optional
     *    * `lastRemovedAt` String - last_removed_at optional
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
     * * `blocking` - blocking optional
     * return: Conversation or null
     * @param {string} topicId
     * @param {boolean | undefined} [blocking]
     * @returns {Promise<any>}
     */
    getConversation(topicId, blocking) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getConversation(this.__wbg_ptr, ptr0, len0, isLikeNone(blocking) ? 0xFFFFFF : blocking ? 1 : 0);
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
     * * `heavy` - heavy optional
     * @param {string} topicId
     * @param {boolean | undefined} [heavy]
     * @returns {Promise<void>}
     */
    setConversationRead(topicId, heavy) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationRead(this.__wbg_ptr, ptr0, len0, isLikeNone(heavy) ? 0xFFFFFF : heavy ? 1 : 0);
        return takeObject(ret);
    }
    /**
     * Set conversation read by topicId
     * #Arguments
     * * `topicId` - topic id
     * * `heavy` - heavy optional
     * @returns {Promise<void>}
     */
    setAllConversationsRead() {
        const ret = wasm.client_setAllConversationsRead(this.__wbg_ptr);
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
     * Clear conversation on local storage
     * #Arguments
     * * `topicId` - topic id
     * @param {string} topicId
     * @returns {Promise<void>}
     */
    clearConversation(topicId) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_clearConversation(this.__wbg_ptr, ptr0, len0);
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
     * * `lastUpdatedAt` - last updated_at
     * * `limit` - limit
     * #Return Array of Conversation
     * #Example
     * ```js
     * const conversations = client.filterConversation((c) => {
     *    return c.remark === 'hello'
     * })
     * ```
     * #Example
     * ```js
     * const conversations = await client.filterConversation((c) => {
     *   return c.remark === 'hello' && c.tags && c.tags.some(t => t.label === 'hello')
     * })
     * @param {any} predicate
     * @param {any} lastUpdatedAt
     * @param {any} limit
     * @returns {Promise<any>}
     */
    filterConversation(predicate, lastUpdatedAt, limit) {
        const ret = wasm.client_filterConversation(this.__wbg_ptr, addHeapObject(predicate), addHeapObject(lastUpdatedAt), addHeapObject(limit));
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
}

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0, 1));

export class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
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
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
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
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0, 1));

export class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
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

const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0, 1));

export class IntoUnderlyingSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_String_8f0eb39a4a4c2f66 = function(arg0, arg1) {
        const ret = String(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_abort_05026c983d86824c = function(arg0) {
        getObject(arg0).abort();
    };
    imports.wbg.__wbg_append_66f7cb821a84ee22 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_append_72d1635ad8643998 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_append_7606a4b52c36db7b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_append_f513a7a3683bdc23 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_arrayBuffer_d0ca2ad8bda0039b = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_bound_324dfb8899c9798c = function() { return handleError(function (arg0, arg1) {
        const ret = IDBKeyRange.bound(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_buffer_61b7ce01341d7f88 = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_dc5dbfa8d5fb28cf = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byobRequest_1fc36a0c1e98611b = function(arg0) {
        const ret = getObject(arg0).byobRequest;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_byteLength_1b2d953758afc500 = function(arg0) {
        const ret = getObject(arg0).byteLength;
        return ret;
    };
    imports.wbg.__wbg_byteOffset_7ef484c6c1d473e9 = function(arg0) {
        const ret = getObject(arg0).byteOffset;
        return ret;
    };
    imports.wbg.__wbg_call_3b770f0d6eb4720e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_500db948e69c7330 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_9bd6f269d4835e33 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3), getObject(arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_b0d8e36992d9900d = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_close_59511bda900d85a8 = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_close_65cb23eb0316f916 = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_commit_fa9822427a46644d = function() { return handleError(function (arg0) {
        getObject(arg0).commit();
    }, arguments) };
    imports.wbg.__wbg_continue_bd44561c014cacea = function() { return handleError(function (arg0) {
        getObject(arg0).continue();
    }, arguments) };
    imports.wbg.__wbg_createIndex_1d4b2bbb6b21b8f8 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).createIndex(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createObjectStore_8d7577746ff46a7d = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).createObjectStore(getStringFromWasm0(arg1, arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_data_4ce8a82394d8b110 = function(arg0) {
        const ret = getObject(arg0).data;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_debug_156ca727dbc3150f = function(arg0) {
        console.debug(getObject(arg0));
    };
    imports.wbg.__wbg_deleteProperty_0ccc7fae163f60ac = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.deleteProperty(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_delete_5c33e4966f59624d = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).delete(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_delete_d97b2b4ff716c553 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).delete();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_done_f22c1561fa919baa = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_enqueue_3997a55771b5212a = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).enqueue(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_entries_4f2bb9b0d701c0f6 = function(arg0) {
        const ret = Object.entries(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_error_fab41a42d22bf2bc = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_fetch_229368eecee9d217 = function(arg0, arg1) {
        const ret = getObject(arg0).fetch(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_fetch_b335d17f45a8b5a1 = function(arg0) {
        const ret = fetch(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getKey_6f6588f340684427 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).getKey(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getTime_ab8b72009983c537 = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_getTimezoneOffset_ec375e661c590c7a = function(arg0) {
        const ret = getObject(arg0).getTimezoneOffset();
        return ret;
    };
    imports.wbg.__wbg_get_6a4f854f5cca7403 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).get(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_9aa3dff3f0266054 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_bbccf8970793c087 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function(arg0, arg1) {
        const ret = getObject(arg0)[getObject(arg1)];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_has_94c2fc1d261bbfe9 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_headers_24e3e19fe3f187c0 = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_host_7131cd3aac9f8fd5 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).host;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_index_871d874253bae760 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).index(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_indexedDB_72e2ca071222fd9e = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).indexedDB;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_info_c3044c86ae29faab = function(arg0) {
        console.info(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_670ddde44cdb2602 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Blob_2fb69097f32d6784 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Blob;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_ErrorEvent_a42064d9628e071c = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ErrorEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Error_2b29c5b4afac4e22 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Error;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_File_788c517d8ef373b5 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof File;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbCursorWithValue_19fe59822a0e6638 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursorWithValue;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbCursor_c78333804645d844 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursor;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbDatabase_4728aafa594bcd0f = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBDatabase;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbOpenDbRequest_a5b41ce7ffc27340 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBOpenDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbRequest_6cd11201baf6632f = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Object_0d0cec232ff037c4 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Object;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Response_d3453657e10c4300 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_28af5bc19d6acad8 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_d2514c6a7ee7ba60 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_XmlHttpRequest_cfe657c299a767a7 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof XMLHttpRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_isArray_1ba11a930108ec51 = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_isSafeInteger_12f5549b2fca23f4 = function(arg0) {
        const ret = Number.isSafeInteger(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_iterator_23604bb983791576 = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_key_87db8226759da642 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).key;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_length_65d1cd11729ced11 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_d65cf0786bfc5739 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_loaded_f83709e2357e370c = function(arg0) {
        const ret = getObject(arg0).loaded;
        return ret;
    };
    imports.wbg.__wbg_location_b2ec7e36fec8a8ff = function(arg0) {
        const ret = getObject(arg0).location;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_log_464d1b2190ca1e04 = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_message_5d9da9f584617c9f = function(arg0, arg1) {
        const ret = getObject(arg1).message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_message_7bde112094278773 = function(arg0) {
        const ret = getObject(arg0).message;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_name_37e12d7b980bc5bd = function(arg0, arg1) {
        const ret = getObject(arg1).name;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_new0_55477545727914d9 = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_079af0206358fe9d = function() { return handleError(function () {
        const ret = new FormData();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_254fa9eac11932ae = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_35d748855c4620b9 = function() { return handleError(function () {
        const ret = new Headers();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_3d446df9155128ef = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_432(a, state0.b, arg0, arg1);
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
    imports.wbg.__wbg_new_3ff5b33b1ce712df = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_41257536af60ed14 = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_5f48f21d4be11586 = function() { return handleError(function () {
        const ret = new AbortController();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_6799ef630abee97c = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_688846f374351c92 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_9b6c38191d7b9512 = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_bc96c6a1c0786643 = function() {
        const ret = new Map();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_c8d37019d16e2945 = function() { return handleError(function () {
        const ret = new XMLHttpRequest();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newnoargs_fd9e4bf8be2bc16d = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_ba35896968751d91 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithstrandinit_a1f6583f20e4faff = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithstrsequenceandoptions_c12c1efe3dd90e2c = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithu8arraysequenceandoptions_75a3b40c32d6c988 = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_next_01dd9234a5bf6d05 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_next_137428deb98342b0 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_objectStore_cdbc73ee600a2cfa = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).objectStore(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_of_437cdae2760f8b94 = function(arg0, arg1) {
        const ret = Array.of(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_openCursor_7d3064e1cd3b1347 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).openCursor(getObject(arg1), __wbindgen_enum_IdbCursorDirection[arg2]);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_openKeyCursor_f59f1d7c19b58c8f = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).openKeyCursor(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_5c51d4d6d7ab6da6 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_78637b05b7fbb2a1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), arg5 !== 0);
    }, arguments) };
    imports.wbg.__wbg_origin_8c23d49bc1f609e9 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).origin;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_push_6edad0df4b546b2c = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_put_78726bde9e67ce9c = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).put(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_queueMicrotask_2181040e064c0dc8 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_queueMicrotask_ef9ac43769cbcc4f = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_random_a435d21390634bdf = function() {
        const ret = Math.random();
        return ret;
    };
    imports.wbg.__wbg_resolve_0bf7c44d641804f9 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_respond_88fe7338392675f2 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_responseText_c8ad7d3362797ebf = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).responseText;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_result_e6ba6a347dcb7470 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).result;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_send_63151ca3486e4507 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).send(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_send_8d1796bdf62d7537 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_setRequestHeader_d685247cc0d080c2 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setRequestHeader(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setTimeout_e202f8ac65a4233b = function(arg0, arg1) {
        setTimeout(getObject(arg0), arg1 >>> 0);
    };
    imports.wbg.__wbg_set_1d80752d0d5f0b21 = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_set_23d69db4e5c66a6e = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    };
    imports.wbg.__wbg_set_76818dc3c59a63d5 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_b554a01e8dc283e8 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).set(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_setbinaryType_3fa4a9e8d2cc506f = function(arg0, arg1) {
        getObject(arg0).binaryType = __wbindgen_enum_BinaryType[arg1];
    };
    imports.wbg.__wbg_setbody_64920df008e48adc = function(arg0, arg1) {
        getObject(arg0).body = getObject(arg1);
    };
    imports.wbg.__wbg_setcredentials_cfc15e48e3a3a535 = function(arg0, arg1) {
        getObject(arg0).credentials = __wbindgen_enum_RequestCredentials[arg1];
    };
    imports.wbg.__wbg_setheaders_4c921e8e226bdfa7 = function(arg0, arg1) {
        getObject(arg0).headers = getObject(arg1);
    };
    imports.wbg.__wbg_setkeypath_ba11b9fa7ea79484 = function(arg0, arg1) {
        getObject(arg0).keyPath = getObject(arg1);
    };
    imports.wbg.__wbg_setmethod_cfc7f688ba46a6be = function(arg0, arg1, arg2) {
        getObject(arg0).method = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setmode_cd03637eb7da01e0 = function(arg0, arg1) {
        getObject(arg0).mode = __wbindgen_enum_RequestMode[arg1];
    };
    imports.wbg.__wbg_setonclose_f9c609d8c9938fa5 = function(arg0, arg1) {
        getObject(arg0).onclose = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_3c78d6b936a761d6 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_72b33e31f9edb045 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_8ae2b387470ec52e = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setonloadend_50e9f4d4b983b8c7 = function(arg0, arg1) {
        getObject(arg0).onloadend = getObject(arg1);
    };
    imports.wbg.__wbg_setonmessage_5e7ade2af360de9d = function(arg0, arg1) {
        getObject(arg0).onmessage = getObject(arg1);
    };
    imports.wbg.__wbg_setonopen_54faa9e83483da1d = function(arg0, arg1) {
        getObject(arg0).onopen = getObject(arg1);
    };
    imports.wbg.__wbg_setonprogress_424fa29a1c436a0e = function(arg0, arg1) {
        getObject(arg0).onprogress = getObject(arg1);
    };
    imports.wbg.__wbg_setonsuccess_57167b1c2650357c = function(arg0, arg1) {
        getObject(arg0).onsuccess = getObject(arg1);
    };
    imports.wbg.__wbg_setontimeout_db8ae5ffb2a7b99f = function(arg0, arg1) {
        getObject(arg0).ontimeout = getObject(arg1);
    };
    imports.wbg.__wbg_setonupgradeneeded_887c7a5fca66011e = function(arg0, arg1) {
        getObject(arg0).onupgradeneeded = getObject(arg1);
    };
    imports.wbg.__wbg_setsignal_f766190d206f09e5 = function(arg0, arg1) {
        getObject(arg0).signal = getObject(arg1);
    };
    imports.wbg.__wbg_settimeout_17ff5302ec75a6c2 = function(arg0, arg1) {
        getObject(arg0).timeout = arg1 >>> 0;
    };
    imports.wbg.__wbg_settype_fd39465d237c2f36 = function(arg0, arg1, arg2) {
        getObject(arg0).type = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setunique_cfc477dc5825e1c4 = function(arg0, arg1) {
        getObject(arg0).unique = arg1 !== 0;
    };
    imports.wbg.__wbg_signal_1fdadeba2d04660e = function(arg0) {
        const ret = getObject(arg0).signal;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_size_5ead5cc358246113 = function(arg0) {
        const ret = getObject(arg0).size;
        return ret;
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_0be7472e492ad3e3 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_1a6eb482d12c9bfb = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_1dc398a895c82351 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_ae1c80c7eea8d64a = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_status_317f53bc4c7638df = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_stringify_f4f701bc34ceda61 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_target_a8fe593e7ee79c21 = function(arg0) {
        const ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_text_dfc4cb7631d2eb34 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).text();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_then_0438fad860fe38e1 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_0ffafeddf0e182a4 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_9422cdd1a30bdfd6 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_a491ccf7be1ca5c9 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_transaction_bc71c2aaaf467420 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).transaction(getStringFromWasm0(arg1, arg2), __wbindgen_enum_IdbTransactionMode[arg3]);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_upload_4c9e54173009f7e6 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).upload;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_url_5327bc0a41a9b085 = function(arg0, arg1) {
        const ret = getObject(arg1).url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_0ad6f37677c8ee74 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_value_4c32fd138a88eee2 = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_view_a03cbb1d55c73e57 = function(arg0) {
        const ret = getObject(arg0).view;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_warn_123db6aa8948382e = function(arg0) {
        console.warn(getObject(arg0));
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +getObject(arg0);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i64 = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
        const ret = BigInt.asUintN(64, arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = getObject(arg1);
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1227 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 661, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1228 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 661, __wbg_adapter_55);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1229 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 661, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1232 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 661, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1235 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 661, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper2234 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 824, __wbg_adapter_64);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = getObject(arg0) in getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_is_bigint = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'bigint';
        return ret;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = getObject(arg0) === null;
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
        const ret = getObject(arg0) === getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = getObject(arg0) == getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('restsend_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
