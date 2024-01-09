import * as wasm from "./restsend_wasm_bg.wasm";
import { __wbg_set_wasm } from "./restsend_wasm_bg.js";
__wbg_set_wasm(wasm);
export * from "./restsend_wasm_bg.js";
