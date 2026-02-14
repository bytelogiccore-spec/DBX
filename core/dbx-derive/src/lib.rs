//! DBX Derive — procedural macros for the DBX database engine.
//!
//! Provides `#[derive(Table)]` for automatic schema generation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive macro for automatic table schema generation.
///
/// # Example
///
/// ```ignore
/// #[derive(Table)]
/// #[dbx(table_name = "users")]
/// pub struct User {
///     #[dbx(primary_key)]
///     pub id: i64,
///     pub name: String,
///     pub email: Option<String>,
/// }
/// ```
///
/// Generates:
/// - `TABLE_NAME` constant
/// - `schema()` → Arrow Schema
/// - `FromRow` trait implementation
#[proc_macro_derive(Table, attributes(dbx))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let table_name = extract_table_name(&input).unwrap_or_else(|| name.to_string().to_lowercase());

    // Extract fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Table can only be derived for structs with named fields"),
        },
        _ => panic!("Table can only be derived for structs"),
    };

    // Generate field information
    let _field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let _field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    // Generate schema fields
    let schema_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap().to_string();
        let field_type = &f.ty;

        quote! {
            arrow::datatypes::Field::new(
                #field_name,
                <#field_type as dbx_core::api::IntoArrowType>::arrow_type(),
                <#field_type as dbx_core::api::IntoArrowType>::is_nullable()
            )
        }
    });

    // Generate FromRow implementation
    let from_row_fields = fields.iter().enumerate().map(|(idx, f)| {
        let field_name = &f.ident;
        let field_type = &f.ty;

        quote! {
            #field_name: <#field_type as dbx_core::api::FromColumn>::from_column(batch.column(#idx), row_idx)?
        }
    });

    let expanded = quote! {
        impl #name {
            pub const TABLE_NAME: &'static str = #table_name;

            pub fn schema() -> arrow::datatypes::Schema {
                arrow::datatypes::Schema::new(vec![
                    #(#schema_fields),*
                ])
            }
        }

        impl dbx_core::api::FromRow for #name {
            fn from_row(batch: &arrow::array::RecordBatch, row_idx: usize) -> dbx_core::error::DbxResult<Self> {
                Ok(Self {
                    #(#from_row_fields),*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_table_name(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if attr.path().is_ident("dbx")
            && let Ok(meta) = attr.parse_args::<syn::Meta>()
            && let syn::Meta::NameValue(nv) = meta
            && nv.path.is_ident("table_name")
            && let syn::Expr::Lit(lit) = nv.value
            && let syn::Lit::Str(s) = lit.lit
        {
            return Some(s.value());
        }
    }
    None
}
