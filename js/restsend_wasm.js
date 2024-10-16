let wasm;

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

let WASM_VECTOR_LEN = 0;

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
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
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
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

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b)
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
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);
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
function __wbg_adapter_52(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h6ecd0e5e6202a828(arg0, arg1);
}

function __wbg_adapter_55(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h0a935f7a0205ce18(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_64(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h662f4660431d9428(arg0, arg1, addHeapObject(arg2));
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

/**
* @param {string | undefined} [level]
*/
export function setLogging(level) {
    var ptr0 = isLikeNone(level) ? 0 : passStringToWasm0(level, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.setLogging(ptr0, len0);
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_408(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h915cca2b2e8f843a(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function notDefined(what) { return () => { throw new Error(`${what} is not defined`); }; }

const ClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_client_free(ptr >>> 0));
/**
*/
export class Client {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_client_free(ptr);
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
    * Sync conversations from server
    * #Arguments
    * * `option` - option
    *    * `syncLogs` - syncs logs, default false
    *    * `syncLogsLimit` - sync logs limit, per conversation, default 100
    *    * `syncLogsMaxCount` - sync logs max count, default 200
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
}

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
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
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
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

const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr);
    }
    /**
    * @param {ReadableStreamDefaultController} controller
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

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

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
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_bigint = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'bigint';
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i64 = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
        const ret = getObject(arg0) === getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = getObject(arg0) in getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
        const ret = BigInt.asUintN(64, arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = getObject(arg0) === null;
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
    imports.wbg.__wbg_setTimeout_a7b6031a86fa1aae = function(arg0, arg1) {
        setTimeout(getObject(arg0), arg1 >>> 0);
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = getObject(arg0) == getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +getObject(arg0);
        return ret;
    };
    imports.wbg.__wbg_String_b9412f8799faab3e = function(arg0, arg1) {
        const ret = String(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_getwithrefkey_edc2c8960f0f1191 = function(arg0, arg1) {
        const ret = getObject(arg0)[getObject(arg1)];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_f975102236d3c502 = function(arg0, arg1, arg2) {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    };
    imports.wbg.__wbg_fetch_1db5b0ae726d68b5 = function(arg0) {
        const ret = fetch(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_3cbae2ec6b6cd3d6 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_481971b0d87f3dd4 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_Window_f401953a2cf86220 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_location_2951b5ee34f19221 = function(arg0) {
        const ret = getObject(arg0).location;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_indexedDB_7c51d9056667f4e0 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).indexedDB;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetch_921fad6ef9e883dd = function(arg0, arg1) {
        const ret = getObject(arg0).fetch(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_debug_5fb96680aecf5dc8 = function(arg0) {
        console.debug(getObject(arg0));
    };
    imports.wbg.__wbg_error_8e3928cfb8a43e2b = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_info_530a29cb2e4e3304 = function(arg0) {
        console.info(getObject(arg0));
    };
    imports.wbg.__wbg_log_5bb5f88f245d7762 = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_warn_63bbae1730aead09 = function(arg0) {
        console.warn(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_IdbDatabase_db671cf2454a9542 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBDatabase;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_createObjectStore_f494613cd1a00d43 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).createObjectStore(getStringFromWasm0(arg1, arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_transaction_1e282a79e9bb7387 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).transaction(getStringFromWasm0(arg1, arg2), takeObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_IdbOpenDbRequest_3f4a166bc0340578 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBOpenDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_setonupgradeneeded_ad7645373c7d5e1b = function(arg0, arg1) {
        getObject(arg0).onupgradeneeded = getObject(arg1);
    };
    imports.wbg.__wbg_loaded_403bea864dd231cc = function(arg0) {
        const ret = getObject(arg0).loaded;
        return ret;
    };
    imports.wbg.__wbg_byobRequest_72fca99f9c32c193 = function(arg0) {
        const ret = getObject(arg0).byobRequest;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_close_184931724d961ccc = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_signal_a61f78a3478fd9bc = function(arg0) {
        const ret = getObject(arg0).signal;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_0d76b0581eca6298 = function() { return handleError(function () {
        const ret = new AbortController();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_abort_2aa7521d5690750e = function(arg0) {
        getObject(arg0).abort();
    };
    imports.wbg.__wbg_instanceof_IdbRequest_93249da04f5370b6 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_result_6cedf5f78600a79c = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).result;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_setonsuccess_632ce0a1460455c2 = function(arg0, arg1) {
        getObject(arg0).onsuccess = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_8479b33e7568a904 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_commit_386dbf9bf88312d5 = function() { return handleError(function (arg0) {
        getObject(arg0).commit();
    }, arguments) };
    imports.wbg.__wbg_objectStore_da468793bd9df17b = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).objectStore(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_view_7f0ce470793a340f = function(arg0) {
        const ret = getObject(arg0).view;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_respond_b1a43b2e3a06d525 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_instanceof_IdbCursorWithValue_abeb44d13d947bc2 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursorWithValue;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_value_86d3334f5075b232 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_XmlHttpRequest_c565494ae9433564 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof XMLHttpRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_settimeout_13085a3f0f4bd4b2 = function(arg0, arg1) {
        getObject(arg0).timeout = arg1 >>> 0;
    };
    imports.wbg.__wbg_upload_60758749ceffcf15 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).upload;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_responseText_c67ed2d48db10769 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).responseText;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    }, arguments) };
    imports.wbg.__wbg_new_71801a555ad9f2aa = function() { return handleError(function () {
        const ret = new XMLHttpRequest();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_c9eb0cf2c9d95679 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), arg5 !== 0);
    }, arguments) };
    imports.wbg.__wbg_send_55ce99107c4b2d47 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).send(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_setRequestHeader_44d235e7a239a771 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setRequestHeader(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_new_d4ab7daa4cb33d5f = function() { return handleError(function () {
        const ret = new FormData();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_append_056476f73715b602 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_append_9fd018eae44ea54a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_append_9c9890ca2ce97dba = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_set_8736c5d85015eea5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).set(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_data_3ce7c145ca4fbcdc = function(arg0) {
        const ret = getObject(arg0).data;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_openCursor_f4453cc6360ffb1c = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).openCursor(getObject(arg1), takeObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_openKeyCursor_a50d16a4992519e2 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).openKeyCursor(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_target_2fc177e386c8b7b0 = function(arg0) {
        const ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_close_a994f9425dab445c = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_enqueue_ea194723156c0cc2 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).enqueue(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_instanceof_Response_849eb93e75734b6e = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_url_5f6dc4009ac5f99d = function(arg0, arg1) {
        const ret = getObject(arg1).url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_status_61a01141acd3cf74 = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_headers_9620bfada380764a = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_arrayBuffer_29931d52c7206b02 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_text_450a059667fd91fd = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).text();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createIndex_d786564b37de8e73 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).createIndex(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_delete_f60bba7d0ae59a4f = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).delete(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_5361b64cac0d0826 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).get(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_index_383b6812c1508030 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).index(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_put_0a0d7ca0f7fa8f83 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).put(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_origin_ee93e29ace71f568 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).origin;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    }, arguments) };
    imports.wbg.__wbg_host_8f1b8ead257c8135 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).host;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    }, arguments) };
    imports.wbg.__wbg_instanceof_ErrorEvent_2dfe691df8ec6f98 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ErrorEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_message_730094b985f3ba2e = function(arg0, arg1) {
        const ret = getObject(arg1).message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_new_ab6fd82b10560829 = function() { return handleError(function () {
        const ret = new Headers();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_append_7bfcb4937d1d5e29 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_open_f0d7259fd7e689ce = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_bound_ce324390556f056d = function() { return handleError(function (arg0, arg1) {
        const ret = IDBKeyRange.bound(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_setonopen_ce7a4c51e5cf5788 = function(arg0, arg1) {
        getObject(arg0).onopen = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_39a785302b0cd2e9 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setonclose_b9929b1c1624dff3 = function(arg0, arg1) {
        getObject(arg0).onclose = getObject(arg1);
    };
    imports.wbg.__wbg_setonmessage_2af154ce83a3dc94 = function(arg0, arg1) {
        getObject(arg0).onmessage = getObject(arg1);
    };
    imports.wbg.__wbg_setbinaryType_b0cf5103cd561959 = function(arg0, arg1) {
        getObject(arg0).binaryType = takeObject(arg1);
    };
    imports.wbg.__wbg_new_6c74223c77cfabad = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_send_70603dff16b81b66 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_setonprogress_b7f97f0b01fe08a7 = function(arg0, arg1) {
        getObject(arg0).onprogress = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_667bee7dd2dad2ed = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setontimeout_e4d7f9d29b30e2b4 = function(arg0, arg1) {
        getObject(arg0).ontimeout = getObject(arg1);
    };
    imports.wbg.__wbg_setonloadend_f3307291000b6737 = function(arg0, arg1) {
        getObject(arg0).onloadend = getObject(arg1);
    };
    imports.wbg.__wbg_instanceof_IdbCursor_e572a9900478e938 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursor;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_key_7a534de95a1f5fbf = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).key;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_continue_f1c3e0815924de62 = function() { return handleError(function (arg0) {
        getObject(arg0).continue();
    }, arguments) };
    imports.wbg.__wbg_delete_21feb1c8d3ec008f = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).delete();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_File_7a8ac75b094722b8 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof File;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_name_f35eb93a73d94973 = function(arg0, arg1) {
        const ret = getObject(arg1).name;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_newwithstrandinit_3fd6fba4083ff2d0 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_Blob_83ad3dd4c9c406f0 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Blob;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_size_9c7e57fbd4f0f4b5 = function(arg0) {
        const ret = getObject(arg0).size;
        return ret;
    };
    imports.wbg.__wbg_newwithu8arraysequenceandoptions_366f462e1b363808 = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithstrsequenceandoptions_ce1f1ca2d522b8aa = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_bd8e338fbd5f5cc8 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_cd7af8117672b8b8 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_new_16b304a2cfa7ff4a = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newnoargs_e258087cd0daa0ea = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_d9bc3a0147634640 = function() {
        const ret = new Map();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_40fc327bfc8770e6 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_196c84450b364254 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_done_298b57d23c0fc80c = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_d93c65011f51a456 = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_iterator_2cee6dadfd956dfa = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_e3c254076557e348 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_27c0f87801dedf93 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_72fb9a18b5ae2624 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_self_ce0dbfc45cf2f5be = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_c6fb939a7f436783 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_d1e6af4856ba331b = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_207b558942527489 = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_d4638f722068f043 = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_isArray_2ab64d95e09ea0ae = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_of_647f9238b4d5407a = function(arg0, arg1) {
        const ret = Array.of(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_push_a5b05aedc7234f9f = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_836825be07d4c9d2 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Error_e20bb56fd5591a93 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Error;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_new_28c511d9baebfa89 = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_message_5bf28016c2b49cfb = function(arg0) {
        const ret = getObject(arg0).message;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_call_b3ca7c6051f9bec1 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_8e7cb608789c2528 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_8417257aaedc936b = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isSafeInteger_f7b04ef02296c4d2 = function(arg0) {
        const ret = Number.isSafeInteger(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_getTime_2bc4375165f02d15 = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_getTimezoneOffset_38257122e236c190 = function(arg0) {
        const ret = getObject(arg0).getTimezoneOffset();
        return ret;
    };
    imports.wbg.__wbg_new_cf3ec55744a78578 = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_7d84e5b2cd9fdc73 = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_Object_71ca3c0a59266746 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Object;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_entries_95cc2c823b285a09 = function(arg0) {
        const ret = Object.entries(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_c816a20ab859d0c1 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_198eafe4f2ea2733 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_81740750da40724f = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_408(a, state0.b, arg0, arg1);
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
    imports.wbg.__wbg_resolve_b0083a7967828ec8 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_0c86a60e8fcfe9f6 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_a73caa9a87991566 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_12d079cc21e14bdb = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_aa4a17c33a06e5cb = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_63b92bc8671ed464 = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_a47bac70306a19a7 = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_length_c20a40f15020d68a = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_2b3bbecd033d19f6 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_buffer_dd7f74bc60f1faab = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byteLength_58f7b4fab1919d44 = function(arg0) {
        const ret = getObject(arg0).byteLength;
        return ret;
    };
    imports.wbg.__wbg_byteOffset_81d60f7392524f62 = function(arg0) {
        const ret = getObject(arg0).byteOffset;
        return ret;
    };
    imports.wbg.__wbg_deleteProperty_13e721a56f19e842 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.deleteProperty(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_has_0af94d20077affa2 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_1f9b04f170055d33 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_stringify_8887fe74e1c50d81 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_random_26e2d782b541ca6b = typeof Math.random == 'function' ? Math.random : notDefined('Math.random');
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = getObject(arg1);
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getBigInt64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? BigInt(0) : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1300 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 646, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1301 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 646, __wbg_adapter_55);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1302 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 646, __wbg_adapter_55);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1303 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 646, __wbg_adapter_55);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1306 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 646, __wbg_adapter_55);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper2205 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 831, __wbg_adapter_64);
        return addHeapObject(ret);
    };

    return imports;
}

function __wbg_init_memory(imports, maybe_memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedBigInt64Memory0 = null;
    cachedFloat64Memory0 = null;
    cachedInt32Memory0 = null;
    cachedUint32Memory0 = null;
    cachedUint8Memory0 = null;


    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(input) {
    if (wasm !== undefined) return wasm;

    if (typeof input === 'undefined') {
        input = new URL('restsend_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await input, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync }
export default __wbg_init;
