extern crate proc_macro;

use proc_macro2;
use crate::proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn;

#[proc_macro_derive(Identifiers)]
pub fn identifiers_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_identifiers(&ast)
}

fn impl_identifiers(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let struct_name = format_ident! {"{}Identifiers", name};
    let vis = &ast.vis;

    let methods = match &ast.data {
        syn::Data::Struct(s) => {
            get_identifiers_methods(&s.fields)
        }
        _ => unreachable!(),
    };

    let gen = quote! {

        #vis struct #struct_name {}

        impl #struct_name {
            #(#methods)*
        }

        impl #name {
            #vis fn idents() -> #struct_name {
                #struct_name{}
            }
        }
    };
    gen.into()
}

fn get_identifiers_methods(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    let mut res = Vec::new();
    for field in fields.iter() {
        if let syn::Visibility::Public(_) = field.vis {
            if let Some(ident) = &field.ident {
                let quote_ident = format!("{}", ident);
                let fun = quote! {
                    pub fn #ident(&self) -> String {
                        String::from(#quote_ident)
                    }
                };
                res.push(fun);
            };
        };
    };
    
    res
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
