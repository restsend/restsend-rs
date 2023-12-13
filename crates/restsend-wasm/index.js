import * as wasm from "./pkg/restsend_wasm_bg.wasm?init";
import { __wbg_set_wasm } from "./pkg/restsend_wasm_bg.js";
__wbg_set_wasm(wasm);
export * from "./pkg/restsend_wasm_bg.js";
