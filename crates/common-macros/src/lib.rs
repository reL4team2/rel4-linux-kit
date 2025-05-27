extern crate syn;

mod entry;
mod ipc;
mod utils;

use darling::{Error, FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, ItemFn, ItemImpl, ItemTrait, parse_quote};

#[derive(Debug, FromMeta)]
struct MacroArgs {
    event: Option<Expr>,
    label: Expr,
}

#[derive(Debug, FromMeta)]
struct IPCTraitArgs {
    event: Expr,
}

// struct ItemFnDeclare {
//     pub attrs: Vec<Attribute>,
//     pub vis: Visibility,
//     pub sig: Signature,
//     pub semicolon: Option<Token![;]>,
// }

// impl Parse for ItemFnDeclare {
//     fn parse(input: ParseStream) -> Result<Self, syn::Error> {
//         Ok(ItemFnDeclare {
//             attrs: input.call(Attribute::parse_outer)?,
//             vis: input.parse()?,
//             sig: input.parse()?,
//             semicolon: input.parse()?,
//         })
//     }
// }

#[proc_macro_attribute]
pub fn generate_ipc_send(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };
    let args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let label = args.label.clone();
    let event = args.event.clone();
    // Parse the trait information and generate the corresponding code
    // 匹配 Trait 并生成对应的代码
    // let input: ItemFnDeclare = syn::parse_macro_input!(input as ItemFnDeclare);
    let input: ItemFn = syn::parse_macro_input!(input as ItemFn);
    ipc::generate_send(input, event, label)
}

#[proc_macro_attribute]
pub fn ipc_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };
    let args = match IPCTraitArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let event_id = args.event.clone();
    // Parse the trait information and generate the corresponding code
    // 匹配 Trait 并生成对应的代码
    // let input: ItemFnDeclare = syn::parse_macro_input!(input as ItemFnDeclare);
    let input: ItemTrait = syn::parse_macro_input!(input as ItemTrait);
    let enum_ident = format_ident!("{}Event", &input.ident);
    let enum_event_id = format_ident!("_{}_eventid", enum_ident);
    let labels: Vec<Ident> = input
        .items
        .iter()
        .filter_map(|x| match x {
            syn::TraitItem::Fn(trait_item_fn) => Some(trait_item_fn.sig.ident.clone()),
            _ => None,
        })
        .collect();

    // The code after expandsion
    // 展开后的代码
    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        pub const #enum_event_id: u64 = #event_id;
        #[allow(non_camel_case_types)]
        #[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
        #[repr(u64)]
        #[derive(Debug)]
        pub enum #enum_ident {
            #(#labels),*
        }

        #input
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn ipc_trait_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input: ItemImpl = syn::parse_macro_input!(input as ItemImpl);
    let trait_path = match input.trait_ {
        Some((_, ref path, _)) => path.clone(),
        None => return TokenStream::from(Error::custom("Need to on a item trait").write_errors()),
    };
    input.items.iter_mut().for_each(|item| {
        if let syn::ImplItem::Fn(impl_item_fn) = item {
            let fname = impl_item_fn.sig.ident.clone();
            impl_item_fn.attrs.push(parse_quote!(
                #[generate_ipc_send(label = #trait_path::#fname)]
            ));
        }
    });
    // The code after expandsion
    // 展开后的代码
    // let expanded = quote! {
    //     #(#attrs)*
    //     #vis #sig {
    //         let mut reg_len: usize = 0;

    //         sel4::with_ipc_buffer_mut(|ib| {
    //             #(#args)*
    //         });

    //         let msg = sel4::MessageInfo::new(#label.into(), 0, 0, reg_len);
    //         let ret = #call_stat;
    //         #output
    //     }
    // };
    let expanded = quote! {
        #input
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input: ItemFn = syn::parse_macro_input!(input as ItemFn);
    let span = input.sig.fn_token.span;
    input.sig.unsafety = Some(syn::token::Unsafe { span });
    quote! {
        #[unsafe(export_name = "_impl_main")]
        #input

    }
    .into()
}
