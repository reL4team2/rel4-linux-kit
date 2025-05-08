extern crate syn;

use darling::{Error, FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Ident, ItemFn, ItemImpl, ItemTrait, PatType, Path, ReturnType, Type, parse_quote,
};

#[derive(Debug, FromMeta)]
struct MacroArgs {
    label: Path,
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

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReturnTypeEnum {
    Number,
    MessageInfo,
    Others,
}

fn get_type(ty: &Box<Type>) -> ReturnTypeEnum {
    match &**ty {
        Type::Path(type_path) => {
            if let Some(ident) = type_path.path.get_ident() {
                let is_num = matches!(
                    ident.to_string().as_str(),
                    "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "usize" | "isize"
                );
                if is_num {
                    ReturnTypeEnum::Number
                } else if matches!(ident.to_string().as_str(), "MessageInfo") {
                    ReturnTypeEnum::MessageInfo
                } else {
                    ReturnTypeEnum::Others
                }
            } else {
                ReturnTypeEnum::Others
            }
        }
        _ => ReturnTypeEnum::Others,
    }
}

fn parse_arg(pat: &PatType) -> proc_macro2::TokenStream {
    let name = pat.pat.clone();
    let pat_ty = get_type(&pat.ty.clone());
    match pat_ty {
        ReturnTypeEnum::Number => quote! {
            ib.msg_regs_mut()[reg_len] = #name as u64;
            reg_len += 1;
        },
        _ => quote! {
            let bytes = #name.as_bytes();
            ib.msg_regs_mut()[reg_len] = bytes.len() as u64;
            let offset = (reg_len + 1) * size_of::<sel4::Word>();
            ib.msg_bytes_mut()[offset..offset + bytes.len()].copy_from_slice(bytes);
            reg_len += bytes.len().div_ceil(size_of::<sel4::Word>()) + 1;
        },
    }
}

fn parse_return(ret_ty: &ReturnType) -> proc_macro2::TokenStream {
    match ret_ty {
        syn::ReturnType::Default => quote! {},
        syn::ReturnType::Type(_, ty) => {
            let ty = get_type(ty);
            match ty {
                ReturnTypeEnum::Number => quote! {
                    sel4::with_ipc_buffer_mut(|ib| ib.msg_regs()[0] as _)
                },
                ReturnTypeEnum::MessageInfo => quote! {ret},
                ReturnTypeEnum::Others => quote! { todo!("Not support non-numeric return type") },
            }
        }
    }
}

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
    // Parse the trait information and generate the corresponding code
    // 匹配 Trait 并生成对应的代码
    // let input: ItemFnDeclare = syn::parse_macro_input!(input as ItemFnDeclare);
    let input: ItemFn = syn::parse_macro_input!(input as ItemFn);
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

    // The code after expandsion
    // 展开后的代码
    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let mut reg_len: usize = 0;

            sel4::with_ipc_buffer_mut(|ib| {
                #(#args)*
            });

            let msg = sel4::MessageInfo::new(#label.into(), 0, 0, reg_len);
            let ret = #call_stat;
            #output
        }
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn ipc_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    // let attr_args = match NestedMeta::parse_meta_list(args.into()) {
    //     Ok(v) => v,
    //     Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    // };
    // let args = match MacroArgs::from_list(&attr_args) {
    //     Ok(v) => v,
    //     Err(e) => return TokenStream::from(e.write_errors()),
    // };
    // Parse the trait information and generate the corresponding code
    // 匹配 Trait 并生成对应的代码
    // let input: ItemFnDeclare = syn::parse_macro_input!(input as ItemFnDeclare);
    let input: ItemTrait = syn::parse_macro_input!(input as ItemTrait);
    let enum_ident = format_ident!("{}Event", &input.ident);
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
        #[allow(non_camel_case_types)]
        #[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
        #[repr(u64)]
        pub enum #enum_ident {
            #(#labels),*
        }

        #input
    };
    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn ipc_trait_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    // let attr_args = match NestedMeta::parse_meta_list(args.into()) {
    //     Ok(v) => v,
    //     Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    // };
    // let args = match MacroArgs::from_list(&attr_args) {
    //     Ok(v) => v,
    //     Err(e) => return TokenStream::from(e.write_errors()),
    // };
    // let label = args.label.clone();
    // Parse the trait information and generate the corresponding code
    // 匹配 Trait 并生成对应的代码
    // let input: ItemFnDeclare = syn::parse_macro_input!(input as ItemFnDeclare);
    let mut input: ItemImpl = syn::parse_macro_input!(input as ItemImpl);
    let trait_path = match input.trait_ {
        Some((_, ref path, _)) => path.clone(),
        None => return TokenStream::from(Error::custom("Need to on a item trait").write_errors()),
    };
    input.items.iter_mut().for_each(|item| match item {
        syn::ImplItem::Fn(impl_item_fn) => {
            impl_item_fn.attrs.push(parse_quote!(#[inline]));
        }
        _ => {}
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
