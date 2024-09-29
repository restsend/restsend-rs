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

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
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
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
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
function __wbg_adapter_52(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h003a83b3a7077f0a(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_57(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hce7ad747f9cd33b1(arg0, arg1);
}

function __wbg_adapter_64(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc06f40e03791573c(arg0, arg1, addHeapObject(arg2));
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

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    const mem = getDataViewMemory0();
    for (let i = 0; i < array.length; i++) {
        mem.setUint32(ptr + 4 * i, addHeapObject(array[i]), true);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_428(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h19a6a499439340f3(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function notDefined(what) { return () => { throw new Error(`${what} is not defined`); }; }

const ClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_client_free(ptr >>> 0, 1));
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
        wasm.__wbg_client_free(ptr, 0);
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
}

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0, 1));
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
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0, 1));
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
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
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
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getwithrefkey_edc2c8960f0f1191 = function(arg0, arg1) {
        const ret = getObject(arg0)[getObject(arg1)];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_f975102236d3c502 = function(arg0, arg1, arg2) {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    };
    imports.wbg.__wbg_fetch_25e3a297f7b04639 = function(arg0) {
        const ret = fetch(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_48421b3cc9052b68 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_12a30234db4045d3 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_Window_5012736c80a01584 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_location_af118da6c50d4c3f = function(arg0) {
        const ret = getObject(arg0).location;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_indexedDB_1f9ee79bddf7d011 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).indexedDB;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetch_ba7fe179e527d942 = function(arg0, arg1) {
        const ret = getObject(arg0).fetch(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_debug_5a33c41aeac15ee6 = function(arg0) {
        console.debug(getObject(arg0));
    };
    imports.wbg.__wbg_error_09480e4aadca50ad = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_info_c261acb2deacd903 = function(arg0) {
        console.info(getObject(arg0));
    };
    imports.wbg.__wbg_log_b103404cc5920657 = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_warn_2b3adb99ce26c314 = function(arg0) {
        console.warn(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_IdbOpenDbRequest_c0d2e9c902441588 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBOpenDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_setonupgradeneeded_8f3f0ac5d7130a6f = function(arg0, arg1) {
        getObject(arg0).onupgradeneeded = getObject(arg1);
    };
    imports.wbg.__wbg_instanceof_IdbRequest_44d99b46adafe829 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_result_fd2dae625828961d = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).result;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_setonsuccess_962c293b6e38a5d5 = function(arg0, arg1) {
        getObject(arg0).onsuccess = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_bd61d0a61808ca40 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_commit_d40764961dd886fa = function() { return handleError(function (arg0) {
        getObject(arg0).commit();
    }, arguments) };
    imports.wbg.__wbg_objectStore_80724f9f6d33ab5b = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).objectStore(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_close_cef2400b120c9c73 = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_enqueue_6f3d433b5e457aea = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).enqueue(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_signal_41e46ccad44bb5e2 = function(arg0) {
        const ret = getObject(arg0).signal;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_ebf2727385ee825c = function() { return handleError(function () {
        const ret = new AbortController();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_abort_8659d889a7877ae3 = function(arg0) {
        getObject(arg0).abort();
    };
    imports.wbg.__wbg_instanceof_IdbDatabase_2c9f91b2db322a72 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBDatabase;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_createObjectStore_cfb780710dbc3ad2 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).createObjectStore(getStringFromWasm0(arg1, arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_deleteObjectStore_745da9b507613eca = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).deleteObjectStore(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_transaction_5a1543682e4ad921 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).transaction(getStringFromWasm0(arg1, arg2), ["readonly","readwrite","versionchange","readwriteflush","cleanup",][arg3]);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_e8f45f3526088828 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_bound_8b7ae17c676052c7 = function() { return handleError(function (arg0, arg1) {
        const ret = IDBKeyRange.bound(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithstrandinit_a31c69e4cc337183 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_loaded_f034229e3662c28f = function(arg0) {
        const ret = getObject(arg0).loaded;
        return ret;
    };
    imports.wbg.__wbg_instanceof_XmlHttpRequest_fa183988f9902cbe = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof XMLHttpRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_settimeout_e534c031fd73fc1d = function(arg0, arg1) {
        getObject(arg0).timeout = arg1 >>> 0;
    };
    imports.wbg.__wbg_upload_5af6070de6734db9 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).upload;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_responseText_ab274b82ca127268 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).responseText;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_new_1ae25a6e3090015c = function() { return handleError(function () {
        const ret = new XMLHttpRequest();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_24ef54e5747f14a4 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), arg5 !== 0);
    }, arguments) };
    imports.wbg.__wbg_send_b43c13474feaca16 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).send(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_setRequestHeader_a83484b96fc24f2e = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setRequestHeader(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_instanceof_IdbCursorWithValue_2302382a73f62174 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursorWithValue;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_value_d4be628e515b251f = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createIndex_6d4c3e20ee0f1066 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).createIndex(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_delete_34764ece57bdc720 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).delete(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_88b5e79e9daccb9f = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).get(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_index_c90226e82bd94b45 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).index(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_put_b697dfdbcfb0598f = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).put(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_ErrorEvent_2a32225473149fc5 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ErrorEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_message_fde1ade05259137c = function(arg0, arg1) {
        const ret = getObject(arg1).message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_instanceof_IdbCursor_df514d970e4cfc91 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof IDBCursor;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_key_37c613728ba0b769 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).key;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_continue_a92b4c9f17458897 = function() { return handleError(function (arg0) {
        getObject(arg0).continue();
    }, arguments) };
    imports.wbg.__wbg_delete_fbab4d55ffb8712b = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).delete();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_origin_648082c4831a5be8 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).origin;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_host_a46347409a9511bd = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).host;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_instanceof_Blob_a959e04f44007d16 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Blob;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_size_8bb43f42080caff8 = function(arg0) {
        const ret = getObject(arg0).size;
        return ret;
    };
    imports.wbg.__wbg_newwithu8arraysequenceandoptions_c8bc456a23f02fca = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithstrsequenceandoptions_f700d764298e22da = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_byobRequest_b32c77640da946ac = function(arg0) {
        const ret = getObject(arg0).byobRequest;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_close_aca7442e6619206b = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_settype_b6ab7b74bd1908a1 = function(arg0, arg1, arg2) {
        getObject(arg0).type = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setkeypath_e6a7c50640d3005a = function(arg0, arg1) {
        getObject(arg0).keyPath = getObject(arg1);
    };
    imports.wbg.__wbg_setbody_734cb3d7ee8e6e96 = function(arg0, arg1) {
        getObject(arg0).body = getObject(arg1);
    };
    imports.wbg.__wbg_setcredentials_2b67800db3f7b621 = function(arg0, arg1) {
        getObject(arg0).credentials = ["omit","same-origin","include",][arg1];
    };
    imports.wbg.__wbg_setheaders_be10a5ab566fd06f = function(arg0, arg1) {
        getObject(arg0).headers = getObject(arg1);
    };
    imports.wbg.__wbg_setmethod_dc68a742c2db5c6a = function(arg0, arg1, arg2) {
        getObject(arg0).method = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setmode_a781aae2bd3df202 = function(arg0, arg1) {
        getObject(arg0).mode = ["same-origin","no-cors","cors","navigate",][arg1];
    };
    imports.wbg.__wbg_setsignal_91c4e8ebd04eb935 = function(arg0, arg1) {
        getObject(arg0).signal = getObject(arg1);
    };
    imports.wbg.__wbg_instanceof_File_f60d4a4d84b71bf1 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof File;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_name_ed3cda975cce080d = function(arg0, arg1) {
        const ret = getObject(arg1).name;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_new_e27c93803e1acc42 = function() { return handleError(function () {
        const ret = new Headers();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_append_f3a4426bb50622c5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setonopen_7e770c87269cae90 = function(arg0, arg1) {
        getObject(arg0).onopen = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_5ec4625df3060159 = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setonclose_40f935717ad6ffcd = function(arg0, arg1) {
        getObject(arg0).onclose = getObject(arg1);
    };
    imports.wbg.__wbg_setonmessage_b670c12ea34acd8b = function(arg0, arg1) {
        getObject(arg0).onmessage = getObject(arg1);
    };
    imports.wbg.__wbg_setbinaryType_d164a0be4c212c9c = function(arg0, arg1) {
        getObject(arg0).binaryType = ["blob","arraybuffer",][arg1];
    };
    imports.wbg.__wbg_new_0bf4a5b0632517ed = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_send_82b52e2f9f8946d9 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_setonprogress_4105d8d164e47b22 = function(arg0, arg1) {
        getObject(arg0).onprogress = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_ac484345c53449bc = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbg_setontimeout_3fd0fc4cd060733f = function(arg0, arg1) {
        getObject(arg0).ontimeout = getObject(arg1);
    };
    imports.wbg.__wbg_setonloadend_95160085ac84bf43 = function(arg0, arg1) {
        getObject(arg0).onloadend = getObject(arg1);
    };
    imports.wbg.__wbg_setunique_6f46c3f803001492 = function(arg0, arg1) {
        getObject(arg0).unique = arg1 !== 0;
    };
    imports.wbg.__wbg_instanceof_Response_e91b7eb7c611a9ae = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_url_1bf85c8abeb8c92d = function(arg0, arg1) {
        const ret = getObject(arg1).url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_status_ae8de515694c5c7c = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_headers_5e283e8345689121 = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_arrayBuffer_a5fbad63cc7e663b = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_text_a94b91ea8700357a = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).text();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_f9f1d655d855a601 = function() { return handleError(function () {
        const ret = new FormData();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_append_876bddfd2c8b42fb = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_append_fc486ec9757bf1c1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_append_b10805b72af15312 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_set_2ce411ff5577e790 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).set(getStringFromWasm0(arg1, arg2), getObject(arg3), getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_data_5c47a6985fefc490 = function(arg0) {
        const ret = getObject(arg0).data;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_view_2a901bda0727aeb3 = function(arg0) {
        const ret = getObject(arg0).view;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_respond_a799bab31a44f2d7 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_target_b7cb1739bee70928 = function(arg0) {
        const ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_openCursor_eae86c5dbc805f16 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).openCursor(getObject(arg1), ["next","nextunique","prev","prevunique",][arg2]);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_openKeyCursor_018dbe2df5dba563 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).openKeyCursor(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_3baa728f9d58d3f6 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_ae22078168b726f5 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_new_a220cf903aa02ca2 = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newnoargs_76313bd6ff35d0f2 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_8608a2b51a5f6737 = function() {
        const ret = new Map();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_de3e9db4440638b2 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_f9cb570345655b9a = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_done_bfda7aa8f252b39f = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_6d39332ab4788d86 = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_iterator_888179a48810a9fe = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_224d16597dbbfd96 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_1084a111329e68ce = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_525245e2b9901204 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_self_3093d5d1f7bcb682 = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_3bcfc4d31bc012f8 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_86b222e13bdf32ed = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_e5a3fe56f8be9485 = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_673dda6c73d19609 = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_isArray_8364a5371e9737d8 = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_of_99c2a118200b1e62 = function(arg0, arg1) {
        const ret = Array.of(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_push_37c89022f34c01ca = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_61dfc3198373c902 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Error_69bde193b0cc95e3 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Error;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_new_796382978dfd4fb0 = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_message_e18bae0a0e2c097a = function(arg0) {
        const ret = getObject(arg0).message;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_call_89af060b4e1523f2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_c6fe275aaa60da79 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_49185437f0ab06f8 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isSafeInteger_7f1ed56200d90674 = function(arg0) {
        const ret = Number.isSafeInteger(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_getTime_91058879093a1589 = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_getTimezoneOffset_c9929a3cc94500fe = function(arg0) {
        const ret = getObject(arg0).getTimezoneOffset();
        return ret;
    };
    imports.wbg.__wbg_new_7982fb43cfca37ae = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_65387337a95cf44d = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_Object_b80213ae6cc9aafb = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Object;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_entries_7a0e06255456ebcd = function(arg0) {
        const ret = Object.entries(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_e17a6671146f47c1 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_9441748cc964d094 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_b85e72ed1bfd57f9 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_428(a, state0.b, arg0, arg1);
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
    imports.wbg.__wbg_resolve_570458cb99d56a43 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_95e6edc0f89b73b1 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_876bb3c633745cc6 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_b7b08af79b0b0974 = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_8a2cb9ca96b27ec9 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_ea1883e1e5e86686 = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_d1e79e2388520f18 = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_length_8339fcf5d8ecd12e = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_247a91427532499e = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_buffer_0710d1b9dbe2eea6 = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byteLength_850664ef28f3e42f = function(arg0) {
        const ret = getObject(arg0).byteLength;
        return ret;
    };
    imports.wbg.__wbg_byteOffset_ea14c35fa6de38cc = function(arg0) {
        const ret = getObject(arg0).byteOffset;
        return ret;
    };
    imports.wbg.__wbg_random_4a6f48b07d1eab14 = typeof Math.random == 'function' ? Math.random : notDefined('Math.random');
    imports.wbg.__wbg_stringify_bbf45426c92a6bf5 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_deleteProperty_9f9c4a61cc6cbf09 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.deleteProperty(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_has_4bfbc01db38743f7 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = getObject(arg1);
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1527 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 673, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1528 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 673, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1529 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 673, __wbg_adapter_57);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1531 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 673, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1533 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 673, __wbg_adapter_52);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper2209 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 819, __wbg_adapter_64);
        return addHeapObject(ret);
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


    if (typeof module !== 'undefined' && Object.getPrototypeOf(module) === Object.prototype)
    ({module} = module)
    else
    console.warn('using deprecated parameters for `initSync()`; pass a single object instead')

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


    if (typeof module_or_path !== 'undefined' && Object.getPrototypeOf(module_or_path) === Object.prototype)
    ({module_or_path} = module_or_path)
    else
    console.warn('using deprecated parameters for the initialization function; pass a single object instead')

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
