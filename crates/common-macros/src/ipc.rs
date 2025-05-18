use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

use crate::utils::{parse_arg, parse_return};

pub fn generate_send(input: ItemFn, event: Option<syn::Expr>, fnid: syn::Expr) -> TokenStream {
    // 检测是否是在 impl 中，如果是在 impl 中，参数会存在 self
    let mut is_impl = false;
    let args: Vec<proc_macro2::TokenStream> = input
        .sig
        .inputs
        .clone()
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat) => Some(parse_arg(pat)),
            _ => {
                is_impl = true;
                None
            }
        })
        .collect();

    let output = parse_return(&input.sig.output);

    let attrs = input.attrs;
    let vis = input.vis;
    let sig = input.sig;

    let call_stat = if is_impl {
        quote!(self.ep.call(msg))
    } else {
        quote!(call_ep!(msg))
    };

    let (event, fn_stat) = match event {
        Some(event) => {
            let stat = quote! {
                reg_len = 1;
                ib.msg_regs_mut()[0] = #fnid;
            };

            (event, stat)
        }
        None => (fnid, quote! {}),
    };

    // The code after expandsion
    // 展开后的代码
    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            use zerocopy::IntoBytes;
            let mut reg_len: usize = 0;

            sel4::with_ipc_buffer_mut(|ib| {
                #fn_stat
                #(#args)*
            });

            let msg = sel4::MessageInfo::new(#event.into(), 0, 0, reg_len);
            let ret = #call_stat;
            #output
        }
    };
    TokenStream::from(expanded)
}
