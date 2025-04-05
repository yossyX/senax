use darling::FromField;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(Debug, FromField)]
#[darling(attributes(sql))]
struct SqlOption {
    #[darling(default)]
    skip: bool,
    #[darling(default)]
    query: Option<String>,
}

#[proc_macro_derive(SqlCol, attributes(sql))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generator(&input) {
        Ok(generated) => generated,
        Err(err) => err.to_compile_error().into(),
    }
}

fn generator(derive_input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let struct_data = match &derive_input.data {
        syn::Data::Struct(v) => v,
        _ => {
            return Err(syn::Error::new_spanned(
                &derive_input.ident,
                "Must be struct type",
            ));
        }
    };

    let mut sql_cols = Vec::new();
    for field in &struct_data.fields {
        let option = SqlOption::from_field(field).unwrap();
        if option.skip {
            continue;
        }
        let ident = field.ident.as_ref().unwrap().to_string();
        let ident = ident.trim_start_matches("r#");
        match option.query {
            Some(query) => {
                sql_cols.push(format!("{} as \"{}\"", query, ident));
            }
            None => {
                sql_cols.push(format!("\"{}\"", ident));
            }
        };
    }
    let sql_cols = proc_macro2::Literal::string(&sql_cols.join(","));

    let struct_name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = &derive_input.generics.split_for_impl();

    let result = quote! {
        impl #impl_generics senax_common::SqlColumns for #struct_name #ty_generics #where_clause {
            fn _sql_cols() -> &'static str {
                #sql_cols
            }
        }
    };
    Ok(result.into())
}
