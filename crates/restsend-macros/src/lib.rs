extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

fn is_wasm_target() -> bool {
    let target = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    match target.as_str() {
        "wasm" => true,
        _ => std::env::var("_").unwrap_or_default().contains("wasm-pack"),
    }
}

#[proc_macro_attribute]
pub fn export_wasm_or_ffi(_metadata: TokenStream, _input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(_input);
    match is_wasm_target() {
        true => {
            quote! {
                #input
            }
        }
        _ => quote! {
            #[uniffi::export]
            #input
        },
    }
    .into()
}
