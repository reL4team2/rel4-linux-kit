#![no_std]
#![no_main]

extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn ipc_msg(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
