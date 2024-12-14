/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const client_set_onconnected: (a: number, b: number) => void;
export const client_set_onconnecting: (a: number, b: number) => void;
export const client_set_ontokenexpired: (a: number, b: number) => void;
export const client_set_onbroken: (a: number, b: number) => void;
export const client_set_onkickoff: (a: number, b: number) => void;
export const client_set_onsystemrequest: (a: number, b: number) => void;
export const client_set_onunknownrequest: (a: number, b: number) => void;
export const client_set_ontopictyping: (a: number, b: number) => void;
export const client_set_ontopicmessage: (a: number, b: number) => void;
export const client_set_ontopicread: (a: number, b: number) => void;
export const client_set_onconversationsupdated: (a: number, b: number) => void;
export const client_set_onconversationsremoved: (a: number, b: number) => void;
export const setLogging: (a: number, b: number) => void;
export const client_createChat: (a: number, b: number, c: number) => number;
export const client_cleanMessages: (a: number, b: number, c: number) => number;
export const client_removeMessages: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_syncChatLogs: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const client_saveChatLogs: (a: number, b: number) => number;
export const client_syncConversations: (a: number, b: number) => number;
export const client_getConversation: (a: number, b: number, c: number, d: number) => number;
export const client_removeConversation: (a: number, b: number, c: number) => number;
export const client_setConversationRemark: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_setConversationSticky: (a: number, b: number, c: number, d: number) => number;
export const client_setConversationMute: (a: number, b: number, c: number, d: number) => number;
export const client_setConversationRead: (a: number, b: number, c: number, d: number) => number;
export const client_setAllConversationsRead: (a: number) => number;
export const client_setConversationTags: (a: number, b: number, c: number, d: number) => number;
export const client_clearConversation: (a: number, b: number, c: number) => number;
export const client_setConversationExtra: (a: number, b: number, c: number, d: number) => number;
export const client_filterConversation: (a: number, b: number, c: number, d: number) => number;
export const client_getUser: (a: number, b: number, c: number, d: number) => number;
export const client_getUsers: (a: number, b: number, c: number) => number;
export const client_setUserRemark: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_setUserStar: (a: number, b: number, c: number, d: number) => number;
export const client_setUserBlock: (a: number, b: number, c: number, d: number) => number;
export const client_setAllowGuestChat: (a: number, b: number) => number;
export const client_createTopic: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const client_joinTopic: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const client_addMember: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_getTopic: (a: number, b: number, c: number) => number;
export const client_getTopicAdmins: (a: number, b: number, c: number) => number;
export const client_getTopicOwner: (a: number, b: number, c: number) => number;
export const client_getTopicMembers: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const client_getTopicKnocks: (a: number, b: number, c: number) => number;
export const client_updateTopic: (a: number, b: number, c: number, d: number) => number;
export const client_updateTopicNotice: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_silentTopic: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_silentTopicMember: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const client_addTopicAdmin: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_removeTopicAdmin: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_transferTopic: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_quitTopic: (a: number, b: number, c: number) => number;
export const client_dismissTopic: (a: number, b: number, c: number) => number;
export const client_acceptTopicJoin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const client_declineTopicJoin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const client_removeTopicMember: (a: number, b: number, c: number, d: number, e: number) => number;
export const signin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
export const signup: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const logout: (a: number, b: number, c: number, d: number) => number;
export const __wbg_client_free: (a: number, b: number) => void;
export const client_new: (a: number, b: number, c: number) => number;
export const client_connectionStatus: (a: number, b: number) => void;
export const client_app_active: (a: number) => void;
export const client_set_keepalive: (a: number, b: number) => void;
export const client_shutdown: (a: number) => number;
export const client_connect: (a: number) => number;
export const client_doSend: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_doTyping: (a: number, b: number, c: number) => number;
export const client_doRecall: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const client_doSendVoice: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_doSendVideo: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_doSendFile: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_doSendLocation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
export const client_doSendLink: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const client_doSendLogs: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
export const client_doSendText: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
export const client_doSendImage: (a: number, b: number, c: number, d: number, e: number) => number;
export const client_doUpdateExtra: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
export const __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
export const intounderlyingbytesource_type: (a: number, b: number) => void;
export const intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
export const intounderlyingbytesource_start: (a: number, b: number) => void;
export const intounderlyingbytesource_pull: (a: number, b: number) => number;
export const intounderlyingbytesource_cancel: (a: number) => void;
export const __wbg_intounderlyingsink_free: (a: number, b: number) => void;
export const intounderlyingsink_write: (a: number, b: number) => number;
export const intounderlyingsink_close: (a: number) => number;
export const intounderlyingsink_abort: (a: number, b: number) => number;
export const __wbg_intounderlyingsource_free: (a: number, b: number) => void;
export const intounderlyingsource_pull: (a: number, b: number) => number;
export const intounderlyingsource_cancel: (a: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __wbindgen_export_3: WebAssembly.Table;
export const __wbindgen_add_to_stack_pointer: (a: number) => number;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h6bd4ca820affe721: (a: number, b: number, c: number) => void;
export const _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h8a4db968a8f99d12: (a: number, b: number) => void;
export const _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h66556920b1f35d36: (a: number, b: number, c: number) => void;
export const wasm_bindgen__convert__closures__invoke2_mut__h3767371f6ec92a1e: (a: number, b: number, c: number, d: number) => void;
