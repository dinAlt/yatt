extern crate proc_macro;

use crate::proc_macro::TokenStream;
use proc_macro2;
use quote::{format_ident, quote};
use syn;

#[proc_macro_derive(Identifiers)]
pub fn identifiers_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let fields: &syn::Fields = match &ast.data {
        syn::Data::Struct(s) => &s.fields,
        _ => unreachable!(),
    };

    let struct_name = &ast.ident;
    let quote_struct_name = format!("{}", &ast.ident);
    let impls_idents = impl_identifiers(&ast.ident, fields, &ast.vis);
    let impls_get_field_val = impl_get_field_val(&ast.ident, fields);
    let field_names = get_fields_list(fields);

    let gen = quote! {
        #impls_idents

        impl yatt_orm::StoreObject for #struct_name {
            fn get_type_name(&self) -> &'static str {
                &#quote_struct_name
            }

            #impls_get_field_val

            fn get_fields_list(&self) -> &'static [&'static str] {
                &[#(&#field_names),*]
            }
        }
    };

    gen.into()
}

fn impl_identifiers(
    name: &syn::Ident,
    fields: &syn::Fields,
    vis: &syn::Visibility,
) -> proc_macro2::TokenStream {
    let methods = get_identifiers_methods(fields, vis);
    quote! {

        // #vis struct #struct_name {}

        impl #name {
            #(#methods)*
        }
    }
}

fn impl_get_field_val(name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream {
    let res: Vec<proc_macro2::TokenStream> = fields
        .iter()
        .filter_map(|f| {
            if let syn::Visibility::Public(_) = f.vis {
                if let Some(ident) = &f.ident {
                    let quote_fn = format!("{}", ident);
                    return Some(quote! {
                        #quote_fn => self.#ident.clone().into(),
                    });
                };
            };
            None
        })
        .collect();

    let quote_name = format!("{}", name);

    quote! {
        fn get_field_val(&self, field_name: &str) -> yatt_orm::FieldVal {
            match field_name {
                #(#res)*
                 _ => panic!(format!("there is no field {} in struct {}", field_name, #quote_name)),
             }
        }
    }
}

fn get_identifiers_methods(
    fields: &syn::Fields,
    vis: &syn::Visibility,
) -> Vec<proc_macro2::TokenStream> {
    let mut res = Vec::new();
    for field in fields.iter() {
        if let syn::Visibility::Public(_) = field.vis {
            if let Some(ident) = &field.ident {
                let fun_name = format_ident!("{}_n", ident);
                let quote_ident = format!("{}", ident);
                let fun = quote! {
                    #vis fn #fun_name() -> &'static str {
                        &#quote_ident
                    }
                };
                res.push(fun);
            };
        };
    }
    res
}

fn get_fields_list(fields: &syn::Fields) -> Vec<String> {
    fields
        .iter()
        .filter_map(|f| {
            if let syn::Visibility::Public(_) = f.vis {
                if let Some(ident) = &f.ident {
                    return Some(format!("{}", ident));
                }
            }

            None
        })
        .collect()
}

// fn impl_field_list(ast: &syn::DeriveInput) -> TokenStream {
//     let name = &ast.ident;
//     let c_name = format_ident!("{}_field_names", name);
//     let fields = match &ast.data {
//         syn::Data::Struct(s) => {
//             let mut res = vec![];
//             for f in s.fields.iter() {
//                 if let syn::Visibility::Public(_) = f.vis {
//                     if let Some(ident) = &f.ident {
//                         res.push(format!("{}", ident));
//                     }
//                 }
//             }
//             res
//         }
//         _ => unreachable!(),
//     };
//     let gen = quote! {
//         const #c_name: &'static [&'static str] = &[#(#fields),*];
//         impl FieldList for #name {
//             fn field_list() -> &'static [&'static str] {
//                 #c_name
//             }
//         }
//     };

//     gen.into()
// }
