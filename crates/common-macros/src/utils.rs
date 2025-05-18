use quote::quote;
use syn::{PatType, ReturnType, Type};

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReturnTypeEnum {
    Number,
    MessageInfo,
    Others,
}

pub fn get_type(ty: &Box<Type>) -> ReturnTypeEnum {
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

pub fn parse_arg(pat: &PatType) -> proc_macro2::TokenStream {
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

pub fn parse_return(ret_ty: &ReturnType) -> proc_macro2::TokenStream {
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
