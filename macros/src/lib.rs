use darling::FromField;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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

    let mut type_mysql = Vec::new();
    let mut type_pgsql = Vec::new();
    let mut type_none = Vec::new();
    for field in &struct_data.fields {
        let option = SqlOption::from_field(field).unwrap();
        if option.skip {
            continue;
        }
        let ident = field.ident.as_ref().unwrap().to_string();
        let ident = ident.trim_start_matches("r#");
        match option.query {
            Some(query) => {
                type_mysql.push(format!("{} as `{}`", query, ident));
                type_pgsql.push(format!("{} as \"{}\"", query, ident));
                type_none.push(format!("{} as {}", query, ident));
            }
            None => {
                type_mysql.push(format!("`{}`", ident));
                type_pgsql.push(format!("\"{}\"", ident));
                type_none.push(ident.to_string());
            }
        };
    }
    let type_mysql = proc_macro2::Literal::string(&type_mysql.join(","));
    let type_pgsql = proc_macro2::Literal::string(&type_pgsql.join(","));
    let type_none = proc_macro2::Literal::string(&type_none.join(","));

    let struct_name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = &derive_input.generics.split_for_impl();

    let gen = quote! {
        impl #impl_generics senax_common::SqlColumns for #struct_name #ty_generics #where_clause {
            fn _sql_cols(quote: &'static str) -> &'static str {
                match quote {
                    "`" => #type_mysql,
                    "\"" => #type_pgsql,
                    _ => #type_none,
                }
            }
        }
    };
    Ok(gen.into())
}
