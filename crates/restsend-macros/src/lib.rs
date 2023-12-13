extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn export_wasm_or_ffi(_metadata: TokenStream, _input: TokenStream) -> TokenStream {
    let mut target = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let input = proc_macro2::TokenStream::from(_input);
    if target.is_empty() {
        let is_wasm_pack = std::env::var("_").unwrap_or_default().contains("wasm-pack");
        target = if is_wasm_pack { "wasm" } else { "" }.to_string();
    }
    match target.as_str() {
        "wasm" => {
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
