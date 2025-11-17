let wasm;

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

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_4.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_5.get(state.dtor)(state.a, state.b)
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
                wasm.__wbindgen_export_5.get(state.dtor)(a, state.b);
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

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    for (let i = 0; i < array.length; i++) {
        const add = addToExternrefTable0(array[i]);
        getDataViewMemory0().setUint32(ptr + 4 * i, add, true);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}
/**
 * Signin with userId and password or token
 * @param {string} endpoint
 * @param {string} userId
 * @param {string | null} [password]
 * @param {string | null} [token]
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
    return ret;
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
    return ret;
}

/**
 * Signup with userId and password
 * @param {string} endpoint
 * @param {string} userId
 * @param {any} extra
 * @returns {Promise<any>}
 */
export function guestLogin(endpoint, userId, extra) {
    const ptr0 = passStringToWasm0(endpoint, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.guestLogin(ptr0, len0, ptr1, len1, extra);
    return ret;
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
    return ret;
}

/**
 * @param {string | null} [level]
 */
export function setLogging(level) {
    var ptr0 = isLikeNone(level) ? 0 : passStringToWasm0(level, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.setLogging(ptr0, len0);
}

function __wbg_adapter_54(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h0d944fd917ac09a7(arg0, arg1);
}

function __wbg_adapter_57(arg0, arg1, arg2) {
    wasm.closure522_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_66(arg0, arg1, arg2) {
    wasm.closure774_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_457(arg0, arg1, arg2, arg3) {
    wasm.closure815_externref_shim(arg0, arg1, arg2, arg3);
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
     * Create a new chat with userId
     * return: Conversation
     * @param {string} userId
     * @returns {Promise<any>}
     */
    createChat(userId) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_createChat(this.__wbg_ptr, ptr0, len0);
        return ret;
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
        return ret;
    }
    /**
     * Remove messages from a conversation
     * @param {string} topicId
     * @param {string[]} chatIds
     * @returns {Promise<void>}
     */
    removeMessages(topicId, chatIds) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArrayJsValueToWasm0(chatIds, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_removeMessages(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
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
     * @param {number | null | undefined} lastSeq
     * @param {any} option
     * @returns {Promise<void>}
     */
    syncChatLogs(topicId, lastSeq, option) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_syncChatLogs(this.__wbg_ptr, ptr0, len0, !isLikeNone(lastSeq), isLikeNone(lastSeq) ? 0 : lastSeq, option);
        return ret;
    }
    /**
     * @param {any} logs
     * @returns {Promise<void>}
     */
    saveChatLogs(logs) {
        const ret = wasm.client_saveChatLogs(this.__wbg_ptr, logs);
        return ret;
    }
    /**
     * Sync conversations from server
     * #Arguments
     * * `option` - option
     *    * `syncMaxCount` - max sync count, default is unlimit
     *    * `syncLogs` - syncs logs, default false
     *    * `syncLogsLimit` - sync logs limit, per conversation, default 100
     *    * `syncLogsMaxCount` - sync logs max count, default 200
     *    * `limit` - limit
     *    * `category` - category optional
     *    * `updatedAt` String - updated_at optional
     *    * `beforeUpdatedAt` String - before_updated_at optional
     *    * `lastRemovedAt` String - last_removed_at optional
     *    * `onsuccess` - onsuccess callback -> function (updated_at:String, count: u32)
     *         - updated_at: last updated_at
     *         - count: count of conversations, if count == limit, there may be more conversations, you can call syncConversations again with updated_at, stop when count < limit
     *    * `onerror` - onerror callback -> function (error: String)
     * @param {any} option
     * @returns {Promise<void>}
     */
    syncConversations(option) {
        const ret = wasm.client_syncConversations(this.__wbg_ptr, option);
        return ret;
    }
    /**
     * @param {any} option
     * @returns {Promise<void>}
     */
    syncFirstPageConversations(option) {
        const ret = wasm.client_syncFirstPageConversations(this.__wbg_ptr, option);
        return ret;
    }
    /**
     * Get conversation by topicId
     * #Arguments
     * * `topicId` - topic id
     * * `blocking` - blocking optional
     * return: Conversation or null
     * @param {string} topicId
     * @param {boolean | null} [blocking]
     * @returns {Promise<any>}
     */
    getConversation(topicId, blocking) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getConversation(this.__wbg_ptr, ptr0, len0, isLikeNone(blocking) ? 0xFFFFFF : blocking ? 1 : 0);
        return ret;
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
        return ret;
    }
    /**
     * Set conversation remark
     * #Arguments
     * * `topicId` - topic id
     * * `remark` - remark
     * @param {string} topicId
     * @param {string | null} [remark]
     * @returns {Promise<any>}
     */
    setConversationRemark(topicId, remark) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(remark) ? 0 : passStringToWasm0(remark, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationRemark(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
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
        return ret;
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
        return ret;
    }
    /**
     * Set conversation read by topicId
     * #Arguments
     * * `topicId` - topic id
     * * `heavy` - heavy optional
     * @param {string} topicId
     * @param {boolean | null} [heavy]
     * @returns {Promise<void>}
     */
    setConversationRead(topicId, heavy) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_setConversationRead(this.__wbg_ptr, ptr0, len0, isLikeNone(heavy) ? 0xFFFFFF : heavy ? 1 : 0);
        return ret;
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
        return ret;
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
        const ret = wasm.client_setConversationTags(this.__wbg_ptr, ptr0, len0, tags);
        return ret;
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
        return ret;
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
        const ret = wasm.client_setConversationExtra(this.__wbg_ptr, ptr0, len0, extra);
        return ret;
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
        const ret = wasm.client_filterConversation(this.__wbg_ptr, predicate, lastUpdatedAt, limit);
        return ret;
    }
    /**
     * Create a new topic
     * #Arguments
     *   name: String,
     *  icon: String,
     * #Return
     * * `Topic` || `undefined`
     * @param {string[]} members
     * @param {string | null} [name]
     * @param {string | null} [icon]
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
        return ret;
    }
    /**
     * Join a topic
     * #Arguments
     * * `topicId` - topic id
     * * `message` - message
     * * `source` - source
     * @param {string} topicId
     * @param {string | null} [message]
     * @param {string | null} [source]
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        const ret = wasm.client_updateTopic(this.__wbg_ptr, ptr0, len0, option);
        return ret;
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
        return ret;
    }
    /**
     * Silence topic
     * #Arguments
     * * `topicId` - topic id
     * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
     * @param {string} topicId
     * @param {string | null} [duration]
     * @returns {Promise<void>}
     */
    silentTopic(topicId, duration) {
        const ptr0 = passStringToWasm0(topicId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(duration) ? 0 : passStringToWasm0(duration, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.client_silentTopic(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Silent topic member
     * #Arguments
     * * `topicId` - topic id
     * * `userId` - user id
     * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
     * @param {string} topicId
     * @param {string} userId
     * @param {string | null} [duration]
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
    }
    /**
     * Accept topic join
     * #Arguments
     * * `topicId` - topic id
     * * `userId` - user id
     * * `memo` - accept memo
     * @param {string} topicId
     * @param {string} userId
     * @param {string | null} [memo]
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
        return ret;
    }
    /**
     * Decline topic join
     * #Arguments
     * * `topicId` - topic id
     * * `userId` - user id
     * * `message` - decline message
     * @param {string} topicId
     * @param {string} userId
     * @param {string | null} [message]
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
        return ret;
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
        return ret;
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
        const ret = wasm.client_doSend(this.__wbg_ptr, ptr0, len0, content, option);
        return ret;
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
        return ret;
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
        const ret = wasm.client_doRecall(this.__wbg_ptr, ptr0, len0, ptr1, len1, option);
        return ret;
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
        const ret = wasm.client_doSendVoice(this.__wbg_ptr, ptr0, len0, attachment, option);
        return ret;
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
        const ret = wasm.client_doSendVideo(this.__wbg_ptr, ptr0, len0, attachment, option);
        return ret;
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
        const ret = wasm.client_doSendFile(this.__wbg_ptr, ptr0, len0, attachment, option);
        return ret;
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
        const ret = wasm.client_doSendLocation(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, option);
        return ret;
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
        const ret = wasm.client_doSendLink(this.__wbg_ptr, ptr0, len0, ptr1, len1, option);
        return ret;
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
     * @param {string[]} logIds
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
        const ret = wasm.client_doSendLogs(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, option);
        return ret;
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
        const ret = wasm.client_doSendText(this.__wbg_ptr, ptr0, len0, ptr1, len1, option);
        return ret;
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
        const ret = wasm.client_doSendImage(this.__wbg_ptr, ptr0, len0, attachment, option);
        return ret;
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
        const ret = wasm.client_doUpdateExtra(this.__wbg_ptr, ptr0, len0, ptr1, len1, extra, option);
        return ret;
    }
    /**
     * Send ping message
     * # Arguments
     * * `content` - The content string
     * * `option` - The send option
     * # Return
     * The message id
     * @param {string} content
     * @param {any} option
     * @returns {Promise<string>}
     */
    doPing(content, option) {
        const ptr0 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_doPing(this.__wbg_ptr, ptr0, len0, option);
        return ret;
    }
    /**
     * Set the callback when connection connected
     * @param {any} cb
     */
    set onconnected(cb) {
        wasm.client_set_onconnected(this.__wbg_ptr, cb);
    }
    /**
     * Set the callback when connection connecting
     * @param {any} cb
     */
    set onconnecting(cb) {
        wasm.client_set_onconnecting(this.__wbg_ptr, cb);
    }
    /**
     * Set the callback when connection token expired
     * @param {any} cb
     */
    set ontokenexpired(cb) {
        wasm.client_set_ontokenexpired(this.__wbg_ptr, cb);
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
        wasm.client_set_onbroken(this.__wbg_ptr, cb);
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
        wasm.client_set_onkickoff(this.__wbg_ptr, cb);
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
        wasm.client_set_onsystemrequest(this.__wbg_ptr, cb);
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
        wasm.client_set_onunknownrequest(this.__wbg_ptr, cb);
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
        wasm.client_set_ontopictyping(this.__wbg_ptr, cb);
    }
    /**
     * Set the callback when receive new message
     * # Arguments
     * * `topicId` String - The topic id
     * * `message` ChatRequest - The message
     * # Return
     * * `hasRead` Boolean - If return true, will send `has read` to server
     * * `unreadCountable` Boolean - If return true, will increase unread count
     * # Example
     * ```javascript
     * const client = new Client(info);
     * await client.connect();
     * client.ontopicmessage = (topicId, message) => {
     * console.log(topicId, message);
     * let hasRead = true;
     * let unreadCountable = message.content?.unreadable !== true
     * return {hasRead, unreadCountable};
     * }
     * ```
     * @param {any} cb
     */
    set ontopicmessage(cb) {
        wasm.client_set_ontopicmessage(this.__wbg_ptr, cb);
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
        wasm.client_set_ontopicread(this.__wbg_ptr, cb);
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
        wasm.client_set_onconversationsupdated(this.__wbg_ptr, cb);
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
        wasm.client_set_onconversationsremoved(this.__wbg_ptr, cb);
    }
    /**
     * Get user info
     * #Arguments
     * * `userId` - user id
     * * `blocking` - blocking fetch from server
     * #Return
     * User info
     * @param {string} userId
     * @param {boolean | null} [blocking]
     * @returns {Promise<any>}
     */
    getUser(userId, blocking) {
        const ptr0 = passStringToWasm0(userId, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getUser(this.__wbg_ptr, ptr0, len0, isLikeNone(blocking) ? 0xFFFFFF : blocking ? 1 : 0);
        return ret;
    }
    /**
     * Get multiple users info
     * #Arguments
     * * `userIds` - Array of user id
     * #Return
     * Array of user info
     * @param {string[]} userIds
     * @returns {Promise<any>}
     */
    getUsers(userIds) {
        const ptr0 = passArrayJsValueToWasm0(userIds, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_getUsers(this.__wbg_ptr, ptr0, len0);
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
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
        return ret;
    }
    /**
     * Create a new client
     * # Arguments
     * * `info` - AuthInfo
     * * `db_name` - database name (optional), create an indexeddb when set it
     * @param {any} info
     * @param {string | null} [db_name]
     */
    constructor(info, db_name) {
        var ptr0 = isLikeNone(db_name) ? 0 : passStringToWasm0(db_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.client_new(info, ptr0, len0);
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
            const ret = wasm.client_connectionStatus(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * get the last alive at
     * @returns {bigint}
     */
    get lastAliveAt() {
        const ret = wasm.client_lastAliveAt(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Promise<number>}
     */
    get unreadCount() {
        const ret = wasm.client_unreadCount(this.__wbg_ptr);
        return ret;
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
     * set the ping interval with seconds (for health check with error logs)
     * default is 30 seconds
     * @param {number} secs
     */
    set ping_interval(secs) {
        wasm.client_set_keepalive(this.__wbg_ptr, secs);
    }
    /**
     * set the max retry count
     * default is 2
     * @param {number} count
     */
    set maxRetry(count) {
        wasm.client_set_maxRetry(this.__wbg_ptr, count);
    }
    /**
     * set the max send idle seconds
     * default is 20 seconds
     * @param {number} secs
     */
    set maxSendIdleSecs(secs) {
        wasm.client_set_maxSendIdleSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the max recall seconds
     * default is 120 seconds
     * note: server may have a limit as well
     * for example, restsend server limit is 2 minutes
     * @param {number} secs
     */
    set maxRecallSecs(secs) {
        wasm.client_set_maxRecallSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the max conversation limit
     * default is 1000
     * note: this limit is for local storage only
     * @param {number} limit
     */
    set maxConversationLimit(limit) {
        wasm.client_set_maxConversationLimit(this.__wbg_ptr, limit);
    }
    /**
     * set the max logs limit per request
     * default is 100
     * note: this limit is for each request to fetch logs from server
     * @param {number} limit
     */
    set maxLogsLimit(limit) {
        wasm.client_set_maxLogsLimit(this.__wbg_ptr, limit);
    }
    /**
     * set the max sync logs max count
     * default is 200
     * note: this limit is for each sync logs operation
     * @param {number} count
     */
    set maxSyncLogsMaxCount(count) {
        wasm.client_set_maxSyncLogsMaxCount(this.__wbg_ptr, count);
    }
    /**
     * set the max connect interval seconds
     * default is 5 seconds
     * @param {number} secs
     */
    set maxConnectIntervalSecs(secs) {
        wasm.client_set_maxConnectIntervalSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the max sync logs limit
     * default is 500
     * @param {number} limit
     */
    set maxSyncLogsLimit(limit) {
        wasm.client_set_maxSyncLogsLimit(this.__wbg_ptr, limit);
    }
    /**
     * set the conversation cache expire seconds
     * default is 60 seconds
     * @param {number} secs
     */
    set conversationCacheExpireSecs(secs) {
        wasm.client_set_conversationCacheExpireSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the user cache expire seconds
     * default is 60 seconds
     * @param {number} secs
     */
    set userCacheExpireSecs(secs) {
        wasm.client_set_userCacheExpireSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the removed conversation cache expire seconds
     * default is 10 seconds
     * @param {number} secs
     */
    set removedConversationCacheExpireSecs(secs) {
        wasm.client_set_removedConversationCacheExpireSecs(this.__wbg_ptr, secs);
    }
    /**
     * set the ping timeout seconds
     * default is 5 seconds
     * @param {number} secs
     */
    set pingTimeoutSecs(secs) {
        wasm.client_set_pingTimeoutSecs(this.__wbg_ptr, secs);
    }
    /**
     * @param {boolean} value
     */
    set build_local_unreadable(value) {
        wasm.client_set_build_local_unreadable(this.__wbg_ptr, value);
    }
    /**
     * @returns {Promise<void>}
     */
    shutdown() {
        const ret = wasm.client_shutdown(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Promise<void>}
     */
    connect() {
        const ret = wasm.client_connect(this.__wbg_ptr);
        return ret;
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
            const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
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
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, controller);
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, controller);
        return ret;
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
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, chunk);
        return ret;
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return ret;
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, reason);
        return ret;
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
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, controller);
        return ret;
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
        const ret = String(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_abort_775ef1d17fc65868 = function(arg0) {
        arg0.abort();
    };
    imports.wbg.__wbg_append_299d5d48292c0495 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_append_8c7dd8d641a5f01b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_append_b2d1fc16de2a0e81 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        arg0.append(getStringFromWasm0(arg1, arg2), arg3, getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_append_b44785ebeb668479 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        arg0.append(getStringFromWasm0(arg1, arg2), arg3);
    }, arguments) };
    imports.wbg.__wbg_apply_36be6a55257c99bf = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.apply(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_arrayBuffer_d1b44c4390db422f = function() { return handleError(function (arg0) {
        const ret = arg0.arrayBuffer();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_bound_55a8d08e0491e17a = function() { return handleError(function (arg0, arg1) {
        const ret = IDBKeyRange.bound(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_buffer_09165b52af8c5237 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_buffer_609cc3eee51ed158 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_byobRequest_77d9adf63337edfb = function(arg0) {
        const ret = arg0.byobRequest;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_byteLength_e674b853d9c77e1d = function(arg0) {
        const ret = arg0.byteLength;
        return ret;
    };
    imports.wbg.__wbg_byteOffset_fd862df290ef848d = function(arg0) {
        const ret = arg0.byteOffset;
        return ret;
    };
    imports.wbg.__wbg_call_672a4d21634d4a24 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_7cccdd69e0791ae2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_833bed5770ea2041 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg0.call(arg1, arg2, arg3);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_close_26fc2e6856d8567a = function(arg0) {
        arg0.close();
    };
    imports.wbg.__wbg_close_2893b7d056a0627d = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_close_304cc1fef3466669 = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_close_5ce03e29be453811 = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_continue_c46c11d3dbe1b030 = function() { return handleError(function (arg0) {
        arg0.continue();
    }, arguments) };
    imports.wbg.__wbg_createIndex_873ac48adc772309 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = arg0.createIndex(getStringFromWasm0(arg1, arg2), arg3, arg4);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createObjectStore_d2f9e1016f4d81b9 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg0.createObjectStore(getStringFromWasm0(arg1, arg2), arg3);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_data_432d9c3df2630942 = function(arg0) {
        const ret = arg0.data;
        return ret;
    };
    imports.wbg.__wbg_debug_3cb59063b29f58c1 = function(arg0) {
        console.debug(arg0);
    };
    imports.wbg.__wbg_deleteProperty_96363d4a1d977c97 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.deleteProperty(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_delete_200677093b4cf756 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.delete(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_delete_2ecf7cf20900b3a2 = function() { return handleError(function (arg0) {
        const ret = arg0.delete();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_done_769e5ede4b31c67b = function(arg0) {
        const ret = arg0.done;
        return ret;
    };
    imports.wbg.__wbg_enqueue_bb16ba72f537dc9e = function() { return handleError(function (arg0, arg1) {
        arg0.enqueue(arg1);
    }, arguments) };
    imports.wbg.__wbg_entries_3265d4158b33e5dc = function(arg0) {
        const ret = Object.entries(arg0);
        return ret;
    };
    imports.wbg.__wbg_error_524f506f44df1645 = function(arg0) {
        console.error(arg0);
    };
    imports.wbg.__wbg_fetch_509096533071c657 = function(arg0, arg1) {
        const ret = arg0.fetch(arg1);
        return ret;
    };
    imports.wbg.__wbg_fetch_b335d17f45a8b5a1 = function(arg0) {
        const ret = fetch(arg0);
        return ret;
    };
    imports.wbg.__wbg_getTime_46267b1c24877e30 = function(arg0) {
        const ret = arg0.getTime();
        return ret;
    };
    imports.wbg.__wbg_getTimezoneOffset_6b5752021c499c47 = function(arg0) {
        const ret = arg0.getTimezoneOffset();
        return ret;
    };
    imports.wbg.__wbg_get_67b2ba62fc30de12 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_get_8da03f81f6a1111e = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.get(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_get_b9b93047fe3cf45b = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return ret;
    };
    imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function(arg0, arg1) {
        const ret = arg0[arg1];
        return ret;
    };
    imports.wbg.__wbg_has_a5ea9117f258a0ec = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_headers_9cb51cfd2ac780a4 = function(arg0) {
        const ret = arg0.headers;
        return ret;
    };
    imports.wbg.__wbg_host_9bd7b5dc07c48606 = function() { return handleError(function (arg0, arg1) {
        const ret = arg1.host;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_index_e00ca5fff206ee3e = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.index(getStringFromWasm0(arg1, arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_indexedDB_b1f49280282046f8 = function() { return handleError(function (arg0) {
        const ret = arg0.indexedDB;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_info_3daf2e093e091b66 = function(arg0) {
        console.info(arg0);
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_e14585432e3737fc = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Blob_ca721ef3bdab15d1 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Blob;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_DomException_ed1ccb7aaf39034c = function(arg0) {
        let result;
        try {
            result = arg0 instanceof DOMException;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_ErrorEvent_24a579ed4d838fe9 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ErrorEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Error_4d54113b22d20306 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Error;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_File_a4e2256bf71955a9 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof File;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbCursorWithValue_18f39d69ed298f6f = function(arg0) {
        let result;
        try {
            result = arg0 instanceof IDBCursorWithValue;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbCursor_4f02b0cddf69c141 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof IDBCursor;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbDatabase_a3ef009ca00059f9 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof IDBDatabase;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbOpenDbRequest_a3416e156c9db893 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof IDBOpenDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbRequest_4813c3f207666aa4 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof IDBRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Object_7f2dcef8f78644a4 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Object;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Response_f2cc20d9f7dfd644 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_17156bcf118086a9 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_def73ea0955fc569 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_XmlHttpRequest_cb46b786b10e467e = function(arg0) {
        let result;
        try {
            result = arg0 instanceof XMLHttpRequest;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_isArray_a1eab7e0d067391b = function(arg0) {
        const ret = Array.isArray(arg0);
        return ret;
    };
    imports.wbg.__wbg_isSafeInteger_343e2beeeece1bb0 = function(arg0) {
        const ret = Number.isSafeInteger(arg0);
        return ret;
    };
    imports.wbg.__wbg_iterator_9a24c88df860dc65 = function() {
        const ret = Symbol.iterator;
        return ret;
    };
    imports.wbg.__wbg_key_29fefecef430db96 = function() { return handleError(function (arg0) {
        const ret = arg0.key;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_length_a446193dc22c12f8 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_e2d2a49132c1b256 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_loaded_d9405d9dd8e0a7e9 = function(arg0) {
        const ret = arg0.loaded;
        return ret;
    };
    imports.wbg.__wbg_location_350d99456c2f3693 = function(arg0) {
        const ret = arg0.location;
        return ret;
    };
    imports.wbg.__wbg_log_c222819a41e063d3 = function(arg0) {
        console.log(arg0);
    };
    imports.wbg.__wbg_message_5c5d919204d42400 = function(arg0, arg1) {
        const ret = arg1.message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_message_97a2af9b89d693a3 = function(arg0) {
        const ret = arg0.message;
        return ret;
    };
    imports.wbg.__wbg_message_d1685a448ba00178 = function(arg0, arg1) {
        const ret = arg1.message;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_name_28c43f147574bf08 = function(arg0, arg1) {
        const ret = arg1.name;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_new0_f788a2397c7ca929 = function() {
        const ret = new Date();
        return ret;
    };
    imports.wbg.__wbg_new_018dcc2d6c8c2f6a = function() { return handleError(function () {
        const ret = new Headers();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_23a2665fac83c611 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_457(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return ret;
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_31a97dac4f10fab7 = function(arg0) {
        const ret = new Date(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_405e22f390576ce2 = function() {
        const ret = new Object();
        return ret;
    };
    imports.wbg.__wbg_new_5e0be73521bc8c17 = function() {
        const ret = new Map();
        return ret;
    };
    imports.wbg.__wbg_new_78feb108b6472713 = function() {
        const ret = new Array();
        return ret;
    };
    imports.wbg.__wbg_new_86231e225ca6b962 = function() { return handleError(function () {
        const ret = new XMLHttpRequest();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_92c54fc74574ef55 = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_9fd39a253424609a = function() { return handleError(function () {
        const ret = new FormData();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_a12002a7f91c75be = function(arg0) {
        const ret = new Uint8Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_c68d7209be747379 = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_new_e25e5aab09ff45db = function() { return handleError(function () {
        const ret = new AbortController();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newnoargs_105ed471475aaf50 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_d97e637ebe145a9a = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwithstrandinit_06c535e0a867c635 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithstrsequenceandoptions_aaff55b467c81b63 = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithu8arraysequenceandoptions_068570c487f69127 = function() { return handleError(function (arg0, arg1) {
        const ret = new Blob(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_next_25feadfc0913fea9 = function(arg0) {
        const ret = arg0.next;
        return ret;
    };
    imports.wbg.__wbg_next_6574e1a8a62d1055 = function() { return handleError(function (arg0) {
        const ret = arg0.next();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_objectStore_21878d46d25b64b6 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.objectStore(getStringFromWasm0(arg1, arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_of_66b3ee656cbd962b = function(arg0, arg1) {
        const ret = Array.of(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbg_openCursor_d8ea5d621ec422f8 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.openCursor(arg1, __wbindgen_enum_IdbCursorDirection[arg2]);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_openKeyCursor_39a119fbf6fa40aa = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.openKeyCursor(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_open_13a598ea50d82926 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4), arg5 !== 0);
    }, arguments) };
    imports.wbg.__wbg_open_e0c0b2993eb596e1 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg0.open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_origin_7c5d649acdace3ea = function() { return handleError(function (arg0, arg1) {
        const ret = arg1.origin;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_push_737cfc8c1432c2c6 = function(arg0, arg1) {
        const ret = arg0.push(arg1);
        return ret;
    };
    imports.wbg.__wbg_put_9ef5363941008835 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.put(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(arg0) {
        queueMicrotask(arg0);
    };
    imports.wbg.__wbg_queueMicrotask_d3219def82552485 = function(arg0) {
        const ret = arg0.queueMicrotask;
        return ret;
    };
    imports.wbg.__wbg_random_3ad904d98382defe = function() {
        const ret = Math.random();
        return ret;
    };
    imports.wbg.__wbg_resolve_4851785c9c5f573d = function(arg0) {
        const ret = Promise.resolve(arg0);
        return ret;
    };
    imports.wbg.__wbg_respond_1f279fa9f8edcb1c = function() { return handleError(function (arg0, arg1) {
        arg0.respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_responseText_ad050aa7f8afec9f = function() { return handleError(function (arg0, arg1) {
        const ret = arg1.responseText;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_result_f29afabdf2c05826 = function() { return handleError(function (arg0) {
        const ret = arg0.result;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_send_0293179ba074ffb4 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_send_873e0cdaab001bca = function() { return handleError(function (arg0, arg1) {
        arg0.send(arg1);
    }, arguments) };
    imports.wbg.__wbg_setRequestHeader_51d371ad5196f6ef = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.setRequestHeader(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setTimeout_77d1545491ae17a4 = function(arg0, arg1) {
        setTimeout(arg0, arg1 >>> 0);
    };
    imports.wbg.__wbg_set_37837023f3d740e8 = function(arg0, arg1, arg2) {
        arg0[arg1 >>> 0] = arg2;
    };
    imports.wbg.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
        arg0[arg1] = arg2;
    };
    imports.wbg.__wbg_set_65595bdd868b3009 = function(arg0, arg1, arg2) {
        arg0.set(arg1, arg2 >>> 0);
    };
    imports.wbg.__wbg_set_8fc6bf8a5b1071d1 = function(arg0, arg1, arg2) {
        const ret = arg0.set(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_set_bc6a9357b130b65e = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        arg0.set(getStringFromWasm0(arg1, arg2), arg3, getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_setbinaryType_92fa1ffd873b327c = function(arg0, arg1) {
        arg0.binaryType = __wbindgen_enum_BinaryType[arg1];
    };
    imports.wbg.__wbg_setbody_5923b78a95eedf29 = function(arg0, arg1) {
        arg0.body = arg1;
    };
    imports.wbg.__wbg_setcredentials_c3a22f1cd105a2c6 = function(arg0, arg1) {
        arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
    };
    imports.wbg.__wbg_setheaders_834c0bdb6a8949ad = function(arg0, arg1) {
        arg0.headers = arg1;
    };
    imports.wbg.__wbg_setkeypath_691179e313c26ae1 = function(arg0, arg1) {
        arg0.keyPath = arg1;
    };
    imports.wbg.__wbg_setmethod_3c5280fe5d890842 = function(arg0, arg1, arg2) {
        arg0.method = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setmode_5dc300b865044b65 = function(arg0, arg1) {
        arg0.mode = __wbindgen_enum_RequestMode[arg1];
    };
    imports.wbg.__wbg_setonclose_14fc475a49d488fc = function(arg0, arg1) {
        arg0.onclose = arg1;
    };
    imports.wbg.__wbg_setonerror_8639efe354b947cd = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonerror_d139b5f89adfb29e = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonerror_d7e3056cc6e56085 = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonloadend_d05c65600b79edb2 = function(arg0, arg1) {
        arg0.onloadend = arg1;
    };
    imports.wbg.__wbg_setonmessage_6eccab530a8fb4c7 = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonopen_2da654e1f39745d5 = function(arg0, arg1) {
        arg0.onopen = arg1;
    };
    imports.wbg.__wbg_setonprogress_0d691cdd71617a18 = function(arg0, arg1) {
        arg0.onprogress = arg1;
    };
    imports.wbg.__wbg_setonsuccess_afa464ee777a396d = function(arg0, arg1) {
        arg0.onsuccess = arg1;
    };
    imports.wbg.__wbg_setontimeout_5e4bb8b9b3a8a3fc = function(arg0, arg1) {
        arg0.ontimeout = arg1;
    };
    imports.wbg.__wbg_setonupgradeneeded_fcf7ce4f2eb0cb5f = function(arg0, arg1) {
        arg0.onupgradeneeded = arg1;
    };
    imports.wbg.__wbg_setsignal_75b21ef3a81de905 = function(arg0, arg1) {
        arg0.signal = arg1;
    };
    imports.wbg.__wbg_settimeout_0354c6307cd5eae8 = function(arg0, arg1) {
        arg0.timeout = arg1 >>> 0;
    };
    imports.wbg.__wbg_settype_39ed370d3edd403c = function(arg0, arg1, arg2) {
        arg0.type = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setunique_dd24c422aa05df89 = function(arg0, arg1) {
        arg0.unique = arg1 !== 0;
    };
    imports.wbg.__wbg_signal_aaf9ad74119f20a4 = function(arg0) {
        const ret = arg0.signal;
        return ret;
    };
    imports.wbg.__wbg_size_3808d41635a9c259 = function(arg0) {
        const ret = arg0.size;
        return ret;
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_status_f6360336ca686bf0 = function(arg0) {
        const ret = arg0.status;
        return ret;
    };
    imports.wbg.__wbg_stringify_f7ed6987935b4a24 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_target_0a62d9d79a2a1ede = function(arg0) {
        const ret = arg0.target;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_text_7805bea50de2af49 = function() { return handleError(function (arg0) {
        const ret = arg0.text();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_then_44b73946d2fb3e7d = function(arg0, arg1) {
        const ret = arg0.then(arg1);
        return ret;
    };
    imports.wbg.__wbg_then_48b406749878a531 = function(arg0, arg1, arg2) {
        const ret = arg0.then(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_toString_5285597960676b7b = function(arg0) {
        const ret = arg0.toString();
        return ret;
    };
    imports.wbg.__wbg_toString_c951aa1c78365ed3 = function(arg0) {
        const ret = arg0.toString();
        return ret;
    };
    imports.wbg.__wbg_transaction_babc423936946a37 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg0.transaction(getStringFromWasm0(arg1, arg2), __wbindgen_enum_IdbTransactionMode[arg3]);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_upload_9e50a6363a8b612e = function() { return handleError(function (arg0) {
        const ret = arg0.upload;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_url_ae10c34ca209681d = function(arg0, arg1) {
        const ret = arg1.url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_68c4e9a54bb7fd5e = function() { return handleError(function (arg0) {
        const ret = arg0.value;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_value_cd1ffa7b1ab794f1 = function(arg0) {
        const ret = arg0.value;
        return ret;
    };
    imports.wbg.__wbg_view_fd8a56e8983f448d = function(arg0) {
        const ret = arg0.view;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_warn_4ca3906c248c47c4 = function(arg0) {
        console.warn(arg0);
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +arg0;
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i64 = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
        const ret = BigInt.asUintN(64, arg0);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = arg1;
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = arg0;
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = arg0.original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1082 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 523, __wbg_adapter_54);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1083 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 523, __wbg_adapter_57);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1084 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 523, __wbg_adapter_57);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1086 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 523, __wbg_adapter_57);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1090 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 523, __wbg_adapter_57);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2293 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 775, __wbg_adapter_66);
        return ret;
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = arg0 in arg1;
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_4;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_is_bigint = function(arg0) {
        const ret = typeof(arg0) === 'bigint';
        return ret;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = arg0 === null;
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = arg0;
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(arg0) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
        const ret = arg0 === arg1;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = arg0 == arg1;
        return ret;
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
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


    wasm.__wbindgen_start();
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
