use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

// <https://docs.rs/syn/latest/syn/>
// <https://github.com/dtolnay/proc-macro-workshop/tree/master/debug>

#[proc_macro_derive(TailQEntry, attributes(entry))]
pub fn tailq_entry_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let Data::Struct(data_struct) = input.data else {
        panic!("struct expected");
    };

    let mut attribute_field = None;
    for field in data_struct.fields {
        for attr in &field.attrs {
            if attr.path().is_ident("entry") {
                attribute_field = Some(field);
                break;
            }
        }
    }

    let attribute_field = attribute_field.expect("missing entry attribute");
    let attribute_field_name = attribute_field.ident.expect("tuple structs unsupported");
    let attribute_field_ty = attribute_field.ty;

    quote! {
        impl ::compat_rs::queue::Entry<#struct_name> for #struct_name {
            unsafe fn entry(this: *mut Self) -> *mut  #attribute_field_ty {
                unsafe { &raw mut (*this).#attribute_field_name }
            }
        }
    }
    .into()
}
